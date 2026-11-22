//! whylog - session walkthroughs and a compacted decision log, so AI agents can read
//! why things were decided.

mod agents;
mod compact;
mod config;
mod json;
mod store;
mod timefmt;

use config::Config;
use std::collections::HashMap;
use std::io::{self, Read};
use std::process::ExitCode;
use store::Session;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const GUIDE: &str = include_str!("../assets/GUIDE.md");
const SKILL: &str = include_str!("../skills/whylog/SKILL.md");
const STYLE: &str = include_str!("../assets/STYLE.md");

const USAGE: &str = "\
whylog - session walkthroughs + a decision log agents can read

usage: whylog <command> [args]

start here
  context                             briefing for an agent starting a task: how to run a
                                      session, writing rules, recent and open sessions

sessions (ID = full id, its 4-hex handle, or any unique part of it)
  open <topic...>                     start a session; prints its handle and file
  note <ID> <text...|->               append a note (\"-\" reads stdin)
  decide <ID> <what - why...>         record a lasting decision for compaction
  touch <ID> <files...>               record files you changed
  close <ID> --summary <text> [--files f...] [--decision d]...
  status [ID]                         open sessions, or one session's header
  list [-n 20]                        newest sessions first
  recent [-n 5]                       recent closed sessions (context for a new task)
  show <ID> | path <ID>               print a walkthrough / its path

decisions
  why <terms...> [-n 20]              search decisions (log + uncompacted sessions)
  compact [--notes 400]               digest of closed sessions not yet compacted
  compact --apply <file|-> --through <ID> [--keep]
                                      append dated decisions, prune compacted walkthroughs

setup
  init [--sessions DIR] [--decisions FILE] [--no-agents]
                                      config, folders, decision log, and the AGENTS.md block
  agents [--check|--remove] [--file F]... add or refresh the whylog block in AGENTS.md
                                      (and CLAUDE.md unless it points to AGENTS.md)
  guide                               the agent workflow guide
  style                               writing rules for notes, decisions and summaries
  skill [--install]                   print the agent skill, or install it to ~/.claude/skills/whylog
  version

flags: --json on open/status/list/recent prints JSON on stdout.
";

struct Args {
    pos: Vec<String>,
    flags: HashMap<String, Vec<String>>,
}

impl Args {
    fn has(&self, f: &str) -> bool {
        self.flags.contains_key(f)
    }
    fn one(&self, f: &str) -> Option<&str> {
        self.flags.get(f).and_then(|v| v.last()).map(String::as_str)
    }
    fn many(&self, f: &str) -> Vec<String> {
        self.flags.get(f).cloned().unwrap_or_default()
    }
    fn num(&self, f: &str, default: usize) -> Result<usize, String> {
        match self.one(f) {
            None => Ok(default),
            Some(v) => v
                .parse()
                .map_err(|_| format!("{f} expects a number, got '{v}'")),
        }
    }
}

const VALUED: &[&str] = &[
    "--summary",
    "--decision",
    "-n",
    "--apply",
    "--through",
    "--notes",
    "--sessions",
    "--decisions",
    "--topic",
];
const LISTS: &[&str] = &["--files", "--file"];
const BOOLS: &[&str] = &[
    "--json",
    "--keep",
    "--install",
    "--help",
    "-h",
    "--quiet",
    "--check",
    "--remove",
    "--no-agents",
];

fn parse(raw: &[String]) -> Result<Args, String> {
    let mut a = Args {
        pos: vec![],
        flags: HashMap::new(),
    };
    let mut i = 0;
    let mut literal = false;
    while i < raw.len() {
        let t = &raw[i];
        if literal || !t.starts_with('-') || t == "-" {
            a.pos.push(t.clone());
        } else if t == "--" {
            literal = true;
        } else if VALUED.contains(&t.as_str()) {
            let v = raw.get(i + 1).ok_or(format!("{t} needs a value"))?;
            a.flags.entry(t.clone()).or_default().push(v.clone());
            i += 1;
        } else if LISTS.contains(&t.as_str()) {
            let e = a.flags.entry(t.clone()).or_default();
            while raw.get(i + 1).is_some_and(|n| !n.starts_with("--")) {
                e.push(raw[i + 1].clone());
                i += 1;
            }
        } else if BOOLS.contains(&t.as_str()) {
            a.flags.entry(t.clone()).or_default();
        } else {
            return Err(format!("unknown flag {t} (see `whylog help`)"));
        }
        i += 1;
    }
    Ok(a)
}

type R = Result<(), String>;

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = raw.first().cloned() else {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    };
    let result = parse(&raw[1..]).and_then(|a| {
        if a.has("--help") || a.has("-h") {
            print!("{USAGE}");
            return Ok(());
        }
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let cfg = config::discover(&cwd);
        run(&cmd, &a, &cfg)
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("whylog: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cmd: &str, a: &Args, cfg: &Config) -> R {
    match cmd {
        "open" => open(a, cfg),
        "note" => note(a, cfg),
        "decide" => decide(a, cfg),
        "touch" => touch(a, cfg),
        "close" => close(a, cfg),
        "status" => status(a, cfg),
        "list" => list(a, cfg, false),
        "recent" => list(a, cfg, true),
        "show" => show(a, cfg, false),
        "path" => show(a, cfg, true),
        "why" => why(a, cfg),
        "compact" => compact_cmd(a, cfg),
        "init" => init(a, cfg),
        "agents" => agents_cmd(a, cfg),
        "context" => context(cfg),
        "guide" => {
            print!("{GUIDE}");
            Ok(())
        }
        "skill" => skill(a),
        "style" => style(cfg),
        "version" | "--version" | "-V" => {
            println!("whylog {}", VERSION.trim());
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command '{other}' (see `whylog help`)")),
    }
}

// ------------------------------------------------------------------ helpers

fn session_arg(a: &Args, cfg: &Config) -> Result<Session, String> {
    let q = a.pos.first().ok_or("missing session ID")?;
    let q = if q == "." {
        std::env::var("WHYLOG_SESSION").map_err(|_| "'.' needs $WHYLOG_SESSION set")?
    } else {
        q.clone()
    };
    store::resolve(cfg, &q)
}

fn text_arg(a: &Args, skip: usize) -> Result<Vec<String>, String> {
    let rest = &a.pos[skip.min(a.pos.len())..];
    if rest.len() == 1 && rest[0] == "-" {
        let mut s = String::new();
        io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| e.to_string())?;
        let lines: Vec<String> = s.lines().map(|l| l.trim_end().to_string()).collect();
        let lines: Vec<String> = lines.into_iter().skip_while(|l| l.is_empty()).collect();
        if lines.iter().all(|l| l.is_empty()) {
            return Err("stdin was empty".into());
        }
        return Ok(lines.iter().map(|l| clean(l)).collect());
    }
    let joined = clean(&rest.join(" "));
    if joined.trim().is_empty() {
        return Err("missing text".into());
    }
    Ok(vec![joined])
}

fn bulletize(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .map(|l| {
            let t = l.trim_start();
            if t.is_empty()
                || t.starts_with("- ")
                || t.starts_with("* ")
                || t.starts_with('#')
                || l.starts_with(' ')
            {
                l
            } else {
                format!("- {l}")
            }
        })
        .collect()
}

fn edit(cfg: &Config, s: &Session, f: impl FnOnce(String) -> String) -> R {
    let _lock = store::lock(cfg).map_err(|e| e.to_string())?;
    let text = std::fs::read_to_string(&s.path).map_err(|e| e.to_string())?;
    store::write_atomic(&s.path, &f(text)).map_err(|e| e.to_string())
}

fn line(s: &Session, cfg: &Config) -> String {
    let date = s
        .closed
        .as_deref()
        .or(s.opened.as_deref())
        .unwrap_or("")
        .get(..10)
        .unwrap_or("");
    format!(
        "{} {:<6} {} {} - {}  [{}]",
        s.short(),
        s.status,
        date,
        s.topic.as_deref().unwrap_or("-"),
        s.summary
            .as_deref()
            .map(|x| x.lines().next().unwrap_or(""))
            .unwrap_or("(no summary)"),
        cfg.rel(&s.path)
    )
}

// ----------------------------------------------------------------- commands

fn open(a: &Args, cfg: &Config) -> R {
    let topic = a
        .one("--topic")
        .map(str::to_string)
        .unwrap_or_else(|| a.pos.join(" "));
    let topic = clean(&topic);
    let _lock = store::lock(cfg).map_err(|e| e.to_string())?;
    let id = store::new_id();
    let path = cfg
        .sessions_dir
        .join(format!("session-{id}-{}.md", store::slugify(&topic)));
    let (secs, _) = timefmt::now();
    store::write_atomic(&path, &store::skeleton(&id, &topic, &timefmt::iso(secs)))
        .map_err(|e| e.to_string())?;
    let s = store::load(&path).map_err(|e| e.to_string())?;
    if a.has("--json") {
        println!("{}", json::session(&s, &cfg.rel(&path)));
    } else {
        println!("opened {} ({}) {}", s.short(), id, cfg.rel(&path));
        if !a.has("--quiet") {
            let h = s.short();
            println!(
                "next: whylog note {h} \"<finding>\" | whylog decide {h} \"**rule** - *reason*\" | whylog close {h} --summary \"<one line>\" --files <changed files>"
            );
        }
    }
    Ok(())
}

fn note(a: &Args, cfg: &Config) -> R {
    let s = session_arg(a, cfg)?;
    let lines = bulletize(text_arg(a, 1)?);
    let n = lines.len();
    edit(cfg, &s, |t| store::append_section(&t, store::NOTES, &lines))?;
    println!(
        "{} +{n} note line{}",
        s.short(),
        if n == 1 { "" } else { "s" }
    );
    Ok(())
}

fn decide(a: &Args, cfg: &Config) -> R {
    let s = session_arg(a, cfg)?;
    let lines = bulletize(text_arg(a, 1)?);
    let n = lines.len();
    edit(cfg, &s, |t| {
        store::append_section(&t, store::DECISIONS, &lines)
    })?;
    println!(
        "{} +{n} decision{}",
        s.short(),
        if n == 1 { "" } else { "s" }
    );
    Ok(())
}

fn merge_files(existing: &[String], new: &[String]) -> Vec<String> {
    let mut v: Vec<String> = existing.to_vec();
    for f in new {
        let f = f.trim().trim_start_matches("./").to_string();
        if !f.is_empty() && !v.contains(&f) {
            v.push(f);
        }
    }
    v.sort();
    v
}

fn touch(a: &Args, cfg: &Config) -> R {
    let s = session_arg(a, cfg)?;
    let new = &a.pos[1..];
    if new.is_empty() {
        return Err("touch needs at least one file".into());
    }
    let files = merge_files(&s.files, new);
    let body: Vec<String> = files.iter().map(|f| format!("- {f}")).collect();
    edit(cfg, &s, |t| store::set_section(&t, store::FILES, &body))?;
    println!("{} files: {}", s.short(), files.len());
    Ok(())
}

fn close(a: &Args, cfg: &Config) -> R {
    let s = session_arg(a, cfg)?;
    if !s.is_open() {
        return Err(format!("session {} is already {}", s.short(), s.status));
    }
    let summary = a
        .one("--summary")
        .ok_or("close needs --summary \"<one line>\"")?
        .trim()
        .to_string();
    let summary = clean(&summary);
    if summary.is_empty() {
        return Err("--summary is empty".into());
    }
    let files = merge_files(&s.files, &a.many("--files"));
    let decisions = bulletize(a.many("--decision").iter().map(|d| clean(d)).collect());
    let (secs, _) = timefmt::now();
    let closed = timefmt::iso(secs);
    let nd = decisions.len();
    edit(cfg, &s, |t| {
        let mut out: Vec<String> = Vec::new();
        let mut done_status = false;
        for l in t.lines() {
            if !done_status && l.trim() == "- Status: open" {
                if !t.contains("\n- Closed:") {
                    out.push(format!("- Closed: {closed}"));
                }
                out.push("- Status: closed".into());
                done_status = true;
            } else {
                out.push(l.to_string());
            }
        }
        let mut t = out.join("\n") + "\n";
        t = store::set_section(&t, store::SUMMARY, std::slice::from_ref(&summary));
        if !decisions.is_empty() {
            t = store::append_section(&t, store::DECISIONS, &decisions);
        }
        let body: Vec<String> = if files.is_empty() {
            vec![store::NONE_MARK.into()]
        } else {
            files.iter().map(|f| format!("- {f}")).collect()
        };
        store::set_section(&t, store::FILES, &body)
    })?;
    let total = s.decisions.len() + nd;
    println!(
        "closed {} - {} file{}, {} decision{}",
        s.short(),
        files.len(),
        if files.len() == 1 { "" } else { "s" },
        total,
        if total == 1 { "" } else { "s" }
    );
    if total == 0 {
        eprintln!(
            "whylog: tip - record lasting decisions with `whylog decide {} \"**what** - *why*\"`",
            s.short()
        );
    }
    if let Some(n) = compaction_due(cfg) {
        eprintln!("whylog: {}", compaction_nudge(cfg, n));
    }
    Ok(())
}

fn status(a: &Args, cfg: &Config) -> R {
    if !a.pos.is_empty() {
        let s = session_arg(a, cfg)?;
        if a.has("--json") {
            println!("{}", json::session(&s, &cfg.rel(&s.path)));
        } else {
            println!("{}", line(&s, cfg));
            if !s.files.is_empty() {
                println!("files: {}", s.files.join(", "));
            }
            for d in &s.decisions {
                println!("decision: {d}");
            }
        }
        return Ok(());
    }
    let open: Vec<Session> = store::all(cfg)
        .into_iter()
        .filter(Session::is_open)
        .collect();
    if a.has("--json") {
        let items: Vec<String> = open
            .iter()
            .map(|s| json::session(s, &cfg.rel(&s.path)))
            .collect();
        println!("[{}]", items.join(","));
    } else if open.is_empty() {
        println!("no open sessions");
    } else {
        for s in &open {
            println!("{}", line(s, cfg));
        }
    }
    Ok(())
}

fn list(a: &Args, cfg: &Config, closed_only: bool) -> R {
    let n = a.num("-n", if closed_only { 5 } else { 20 })?;
    let mut v: Vec<Session> = store::all(cfg)
        .into_iter()
        .filter(|s| !closed_only || !s.is_open())
        .collect();
    v.sort_by(|x, y| y.id.cmp(&x.id));
    v.truncate(n);
    if a.has("--json") {
        let items: Vec<String> = v
            .iter()
            .map(|s| json::session(s, &cfg.rel(&s.path)))
            .collect();
        println!("[{}]", items.join(","));
        return Ok(());
    }
    if v.is_empty() {
        println!("no sessions");
    }
    for s in &v {
        println!("{}", line(s, cfg));
    }
    Ok(())
}

fn show(a: &Args, cfg: &Config, path_only: bool) -> R {
    let s = session_arg(a, cfg)?;
    if path_only {
        println!("{}", cfg.rel(&s.path));
    } else {
        print!(
            "{}",
            std::fs::read_to_string(&s.path).map_err(|e| e.to_string())?
        );
    }
    Ok(())
}

fn why(a: &Args, cfg: &Config) -> R {
    let terms: Vec<String> = a.pos.iter().map(|t| t.to_lowercase()).collect();
    if terms.is_empty() {
        return Err("why needs search terms, e.g. `whylog why retry http`".into());
    }
    let n = a.num("-n", 20)?;
    let matches = |l: &str| {
        let low = l.to_lowercase();
        terms.iter().all(|t| low.contains(t))
    };
    let mut hits: Vec<String> = Vec::new();
    let rel = cfg.rel(&cfg.decisions_file);
    if let Ok(text) = std::fs::read_to_string(&cfg.decisions_file) {
        let mut heading = String::new();
        for (i, l) in text.lines().enumerate() {
            if let Some(h) = l.strip_prefix("## ").or_else(|| l.strip_prefix("### ")) {
                heading = h.trim().to_string();
                continue;
            }
            if l.trim_start().starts_with("- ") && matches(l) {
                hits.push(format!("{rel}:{} [{heading}] {}", i + 1, l.trim()));
            }
        }
    }
    for s in store::all(cfg) {
        for d in &s.decisions {
            if matches(d) {
                hits.push(format!(
                    "session {} [{}] - {d}",
                    s.short(),
                    s.topic.as_deref().unwrap_or("-")
                ));
            }
        }
    }
    if hits.is_empty() {
        println!("no decisions match: {}", a.pos.join(" "));
        return Ok(());
    }
    let total = hits.len();
    for h in hits.iter().take(n) {
        println!("{h}");
    }
    if total > n {
        println!("… {} more (raise -n or narrow the terms)", total - n);
    }
    Ok(())
}

fn compact_cmd(a: &Args, cfg: &Config) -> R {
    let decisions = std::fs::read_to_string(&cfg.decisions_file).unwrap_or_default();
    let mark = compact::marker(&decisions);
    let all = store::all(cfg);
    let Some(src) = a.one("--apply") else {
        let p = compact::pending(&all, mark.as_deref());
        print!("{}", compact::digest(&p, a.num("--notes", 400)?));
        return Ok(());
    };

    let input = if src == "-" {
        let mut s = String::new();
        io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| e.to_string())?;
        s
    } else {
        std::fs::read_to_string(src).map_err(|e| format!("{src}: {e}"))?
    };
    let lines = compact::parse_lines(&clean(&input));

    let _lock = store::lock(cfg).map_err(|e| e.to_string())?;
    let pending = compact::pending(&all, mark.as_deref());
    let through = match a.one("--through") {
        Some(t) => store::resolve(cfg, t).map(|s| s.id).or_else(|e| {
            // the session may already be pruned by a concurrent apply; accept a full id verbatim
            if t.contains('T') {
                Ok(t.to_string())
            } else {
                Err(e)
            }
        })?,
        None => return Err("--apply needs --through <ID> (printed by `whylog compact`)".into()),
    };
    let covered: Vec<&Session> = pending.into_iter().filter(|s| s.id <= through).collect();
    if covered.is_empty() {
        return Err("nothing to compact up to that id".into());
    }
    let (secs, _) = timefmt::now();
    let base = if decisions.trim().is_empty() {
        let name = cfg
            .root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Project".into());
        compact::decisions_skeleton(&name)
    } else {
        decisions.clone()
    };
    let entry_lines = if lines.is_empty() {
        vec![(
            None,
            "- _no lasting decisions in these sessions_".to_string(),
        )]
    } else {
        lines.clone()
    };
    let updated = compact::append_entry(&base, &timefmt::date(secs), covered.len(), &entry_lines);
    let updated = compact::set_marker(&updated, &through);
    store::write_atomic(&cfg.decisions_file, &updated).map_err(|e| e.to_string())?;

    let mut pruned = 0;
    if !a.has("--keep") {
        for s in &covered {
            if std::fs::remove_file(&s.path).is_ok() {
                pruned += 1;
            }
        }
    }
    println!(
        "compacted {} session{} → {} (+{} decision{}){}",
        covered.len(),
        if covered.len() == 1 { "" } else { "s" },
        cfg.rel(&cfg.decisions_file),
        lines.len(),
        if lines.len() == 1 { "" } else { "s" },
        if a.has("--keep") {
            String::new()
        } else {
            format!(
                "; pruned {pruned} walkthrough{}",
                if pruned == 1 { "" } else { "s" }
            )
        }
    );
    Ok(())
}

fn init(a: &Args, cfg: &Config) -> R {
    let root = if cfg.from_file {
        cfg.root.clone()
    } else {
        std::env::current_dir().map_err(|e| e.to_string())?
    };
    let cfg_path = root.join(config::CONFIG_FILE);
    let sessions = a.one("--sessions").unwrap_or(config::DEFAULT_SESSIONS_DIR);
    let decisions = a
        .one("--decisions")
        .unwrap_or(config::DEFAULT_DECISIONS_FILE);
    let mut made: Vec<String> = Vec::new();
    if !cfg_path.exists() {
        std::fs::write(&cfg_path, config::render(sessions, decisions))
            .map_err(|e| e.to_string())?;
        made.push(config::CONFIG_FILE.into());
    }
    let cfg = config::from_text(
        &root,
        &std::fs::read_to_string(&cfg_path).unwrap_or_default(),
        true,
    );
    if !cfg.sessions_dir.exists() {
        std::fs::create_dir_all(&cfg.sessions_dir).map_err(|e| e.to_string())?;
        made.push(cfg.rel(&cfg.sessions_dir) + "/");
    }
    if !cfg.decisions_file.exists() {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Project".into());
        store::write_atomic(&cfg.decisions_file, &compact::decisions_skeleton(&name))
            .map_err(|e| e.to_string())?;
        made.push(cfg.rel(&cfg.decisions_file));
    }
    if !a.has("--no-agents") {
        for line in sync_agents(&cfg, &agent_targets(&cfg, &[]))? {
            made.push(line);
        }
    }
    if made.is_empty() {
        println!(
            "already initialized ({}, {})",
            cfg.rel(&cfg.sessions_dir),
            cfg.rel(&cfg.decisions_file)
        );
    } else {
        println!("created/updated {}", made.join(", "));
        println!("agents will find whylog in AGENTS.md and start each task with `whylog context`");
    }
    Ok(())
}

fn skill(a: &Args) -> R {
    if !a.has("--install") {
        print!("{SKILL}");
        return Ok(());
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or("cannot find your home directory (set HOME)")?;
    let dir = std::path::PathBuf::from(home)
        .join(".claude")
        .join("skills")
        .join("whylog");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("SKILL.md");
    std::fs::write(&path, SKILL).map_err(|e| e.to_string())?;
    println!("installed {}", path.display());
    Ok(())
}

/// House style: no em/en dashes in anything whylog writes.
fn clean(s: &str) -> String {
    s.replace(" \u{2014} ", " - ")
        .replace(" \u{2013} ", " - ")
        .replace(['\u{2014}', '\u{2013}'], "-")
}

fn style(cfg: &Config) -> R {
    print!("{STYLE}");
    if let Some(p) = &cfg.style_file {
        match std::fs::read_to_string(p) {
            Ok(extra) => print!("\n## Project rules ({})\n\n{extra}", cfg.rel(p)),
            Err(e) => eprintln!("whylog: style_file {}: {e}", cfg.rel(p)),
        }
    }
    Ok(())
}

// ------------------------------------------------------------- compaction nudge

/// Closed sessions waiting to be compacted, if that's at or over the project threshold.
fn compaction_due(cfg: &Config) -> Option<usize> {
    if cfg.compact_after == 0 {
        return None;
    }
    let log = std::fs::read_to_string(&cfg.decisions_file).unwrap_or_default();
    let n = compact::pending(&store::all(cfg), compact::marker(&log).as_deref()).len();
    (n >= cfg.compact_after).then_some(n)
}

fn compaction_nudge(cfg: &Config, n: usize) -> String {
    format!(
        "{n} closed sessions are waiting to be compacted (the project compacts at {}). \
         Recommend compacting: `whylog compact`, write the lasting decisions, then \
         `whylog compact --apply <file> --through <id>`. If you're in the middle of the user's \
         task, finish it first and suggest it.",
        cfg.compact_after
    )
}

// ------------------------------------------------------------- agent setup

/// AGENTS.md always; CLAUDE.md too when it exists and doesn't point to AGENTS.md.
fn agent_targets(cfg: &Config, explicit: &[String]) -> Vec<std::path::PathBuf> {
    if !explicit.is_empty() {
        return explicit.iter().map(|f| cfg.root.join(f)).collect();
    }
    let mut v = vec![cfg.root.join("AGENTS.md")];
    let claude = cfg.root.join("CLAUDE.md");
    if let Ok(text) = std::fs::read_to_string(&claude)
        && !agents::imports_agents(&text)
    {
        v.push(claude);
    }
    v
}

/// Write or refresh the block in each target. Returns a line per file changed.
fn sync_agents(cfg: &Config, targets: &[std::path::PathBuf]) -> Result<Vec<String>, String> {
    let block = agents::block(cfg);
    let mut changed = Vec::new();
    for t in targets {
        let before = std::fs::read_to_string(t).unwrap_or_default();
        let after = agents::inject(&before, &block);
        if after != before {
            std::fs::write(t, &after).map_err(|e| format!("{}: {e}", cfg.rel(t)))?;
            changed.push(format!("{} (whylog block)", cfg.rel(t)));
        }
    }
    Ok(changed)
}

fn agents_cmd(a: &Args, cfg: &Config) -> R {
    let targets = agent_targets(cfg, &a.many("--file"));
    let block = agents::block(cfg);
    if a.has("--check") {
        let mut bad = Vec::new();
        for t in &targets {
            let text = std::fs::read_to_string(t).unwrap_or_default();
            match agents::current(&text) {
                Some(cur) if cur == block => {}
                Some(_) => bad.push(format!("{} (stale block)", cfg.rel(t))),
                None => bad.push(format!("{} (no whylog block)", cfg.rel(t))),
            }
        }
        if bad.is_empty() {
            println!("agent instructions up to date");
            return Ok(());
        }
        return Err(format!("run `whylog agents` to fix: {}", bad.join(", ")));
    }
    if a.has("--remove") {
        for t in &targets {
            if let Ok(text) = std::fs::read_to_string(t)
                && agents::has_block(&text)
            {
                std::fs::write(t, agents::remove(&text)).map_err(|e| e.to_string())?;
                println!("removed whylog block from {}", cfg.rel(t));
            }
        }
        return Ok(());
    }
    let changed = sync_agents(cfg, &targets)?;
    if changed.is_empty() {
        println!("agent instructions already up to date");
    } else {
        println!("updated {}", changed.join(", "));
    }
    Ok(())
}

fn context(cfg: &Config) -> R {
    let project = cfg
        .root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "this project".into());
    let decisions_rel = cfg.rel(&cfg.decisions_file);
    println!("# whylog context: {project}\n");
    println!(
        "This repo records every task as a whylog session. Lasting decisions and the reasons\n\
         behind them are kept in `{decisions_rel}`. Read what applies to your task before you\n\
         change anything, and leave a record of what you did and why.\n"
    );
    print!("{}", agents::BRIEFING);
    if let Some(p) = &cfg.style_file {
        println!(
            "Project writing rules: `{}` (included in `whylog style`).",
            cfg.rel(p)
        );
    }

    let log = std::fs::read_to_string(&cfg.decisions_file).unwrap_or_default();
    let count = log
        .lines()
        .filter(|l| l.trim_start().starts_with("- **"))
        .count();
    let last = log
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("## Compaction "))
        .and_then(|r| r.split_whitespace().next())
        .map(str::to_string)
        .or_else(|| {
            // fall back to the marker's id (YYYYMMDDT...) when there's no dated heading yet
            compact::marker(&log).and_then(|m| {
                m.get(..8)
                    .map(|d| format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..8]))
            })
        })
        .unwrap_or_else(|| "never".into());
    println!("\n## Decision log\n");
    if log.is_empty() {
        println!("`{decisions_rel}` doesn't exist yet. Run `whylog init`.");
    } else {
        println!(
            "`{decisions_rel}`: {count} decisions, last compacted {last}. Search it with `whylog why <words>`."
        );
    }

    let mut all = store::all(cfg);
    all.sort_by(|x, y| y.id.cmp(&x.id));
    let open: Vec<&Session> = all.iter().filter(|s| s.is_open()).collect();
    if !open.is_empty() {
        println!(
            "\n## Open sessions ({}), others may be working right now\n",
            open.len()
        );
        for s in open.iter().take(10) {
            println!("{}", line(s, cfg));
        }
    }
    let recent: Vec<&Session> = all.iter().filter(|s| !s.is_open()).take(5).collect();
    if !recent.is_empty() {
        println!("\n## Recent sessions\n");
        for s in recent {
            println!("{}", line(s, cfg));
        }
    }
    if let Some(n) = compaction_due(cfg) {
        println!("\n## Compaction due\n\n{}", compaction_nudge(cfg, n));
    }
    if agent_targets(cfg, &[])
        .iter()
        .any(|t| !agents::has_block(&std::fs::read_to_string(t).unwrap_or_default()))
    {
        println!(
            "\nNote: AGENTS.md has no whylog block yet. Run `whylog agents` so every agent sees these rules."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_docs_ship_in_the_binary() {
        // include_str! compiles these into the binary; this guards against them going empty.
        assert!(super::STYLE.contains("No em-dashes"));
        assert!(super::GUIDE.contains("whylog style"));
        assert!(super::SKILL.starts_with("---\nname: whylog"));
    }

    #[test]
    fn clean_strips_dashes() {
        assert_eq!(super::clean("**a** \u{2014} *b*"), "**a** - *b*");
        assert_eq!(super::clean("x\u{2014}y \u{2013} z"), "x-y - z");
    }
}
