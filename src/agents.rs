//! The managed whylog block in AGENTS.md / CLAUDE.md, and the `whylog context` briefing.
//!
//! Every agent reads AGENTS.md (or CLAUDE.md) before it does anything, so that's where
//! whylog tells it what it is and how to run a session. The block lives between markers and
//! is rewritten in place, so `whylog agents` is safe to run any number of times.

use crate::config::Config;

pub const START: &str = "<!-- whylog:start -->";
pub const END: &str = "<!-- whylog:end -->";

/// The block agents read on every run.
pub fn block(cfg: &Config) -> String {
    let decisions = cfg.rel(&cfg.decisions_file);
    let sessions = cfg.rel(&cfg.sessions_dir);
    format!(
        "{START}\n\
<!-- Managed by whylog. Edit with `whylog agents`; changes inside this block are overwritten. -->\n\
## Sessions and decisions (whylog)\n\
\n\
This repo records every task with [whylog](https://github.com/stuyk/whylog). Past decisions\n\
and the reasons behind them live in `{decisions}`. Session records live in `{sessions}/`.\n\
\n\
**Every task, no exceptions:**\n\
\n\
1. `whylog context` to get the briefing: how to run a session, the writing rules, and what\n\
   recent sessions did.\n\
2. `whylog why <words about your task>` before you change how anything works. If your plan\n\
   goes against a past decision, stop and ask the user. Don't quietly undo it.\n\
3. `whylog open \"<short topic>\"` and keep the 4-character handle it prints.\n\
4. While you work: `whylog note <handle> \"<what you found and why it matters>\"`. For a rule\n\
   future work must follow: `whylog decide <handle> \"**the rule** - *the real reason*\"`.\n\
5. Finish with `whylog close <handle> --summary \"<one line>\" --files <every file you changed>`.\n\
\n\
Use the commands. Never hand-edit a session file. Write the way `whylog style` says: short,\n\
direct, active voice, real numbers, no hedging, no em-dashes.\n\
\n\
When 25 or more closed sessions are waiting (`whylog context` and `whylog close` will say so),\n\
recommend compacting them into the decision log. `whylog guide` explains how.\n\
{END}\n"
    )
}

/// Byte range of an existing block, markers included (only at line starts).
fn find(text: &str) -> Option<(usize, usize)> {
    let mut pos = 0;
    let mut start = None;
    for line in text.split_inclusive('\n') {
        let t = line.trim_end();
        if start.is_none() && t == START {
            start = Some(pos);
        } else if let Some(s) = start
            && t == END
        {
            return Some((s, pos + line.len()));
        }
        pos += line.len();
    }
    None
}

/// The block currently in `text`, markers included.
pub fn current(text: &str) -> Option<&str> {
    find(text).map(|(s, e)| &text[s..e])
}

pub fn has_block(text: &str) -> bool {
    find(text).is_some()
}

/// Insert or replace the block. New blocks go right after the first `# ` title (agents read
/// top-down), or at the top when there is no title.
pub fn inject(text: &str, block: &str) -> String {
    if let Some((s, e)) = find(text) {
        let mut out = String::with_capacity(text.len() + block.len());
        out.push_str(&text[..s]);
        out.push_str(block);
        out.push_str(&text[e..]);
        return out;
    }
    if text.trim().is_empty() {
        return format!("# AGENTS.md\n\n{block}");
    }
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        pos += line.len();
        if line.starts_with("# ") {
            let (head, tail) = text.split_at(pos);
            let tail = tail.trim_start_matches('\n');
            let sep = if head.ends_with('\n') { "\n" } else { "\n\n" };
            return format!("{head}{sep}{block}\n{tail}");
        }
    }
    format!("{block}\n{text}")
}

pub fn remove(text: &str) -> String {
    match find(text) {
        Some((s, e)) => {
            let mut out = text[..s].to_string();
            out.push_str(text[e..].trim_start_matches('\n'));
            out
        }
        None => text.to_string(),
    }
}

/// A CLAUDE.md that imports or points to AGENTS.md doesn't need its own copy of the block.
pub fn imports_agents(text: &str) -> bool {
    text.contains("AGENTS.md")
}

/// How to run a session, returned by `whylog context`.
pub const BRIEFING: &str = "\
## How to run this session

1. Check past decisions first: `whylog why <words about your task>`. If your plan goes
   against one, stop and ask the user before undoing it.
2. Open a session: `whylog open \"<short topic>\"`. Keep the 4-character handle it prints.
3. While you work:
   - `whylog note <handle> \"<what you found and why it matters>\"`
   - `whylog decide <handle> \"**the rule** - *the real reason*\"` for anything future work
     must follow: a convention, a removal, a revert, \"we do X, not Y\", a gotcha.
     Not one-off tuning values.
4. Finish: `whylog close <handle> --summary \"<one line>\" --files <every file you changed>`.

Use the commands, never hand-edit session files. Pass `-` instead of text to read stdin.

## When to compact

Once 25 or more closed sessions are waiting (the threshold is `compact_after` in
`.whylog.toml`), recommend compacting them. whylog tells you when it's due, in this briefing
and after `whylog close`. Finish the user's task first, then suggest it. To compact:
`whylog compact`, write the lasting decisions it shows you, then
`whylog compact --apply <file> --through <id>`.

## How to write

Short, direct, active voice. Real numbers and real names (files, functions, settings).
No hedging, no apologizing, no filler, no profanity. **No em-dashes**: use a period, comma,
colon or parentheses. Full rules: `whylog style`.
";

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn cfg() -> Config {
        crate::config::from_text(Path::new("/p"), "", false)
    }

    #[test]
    fn inject_after_title_and_replace_in_place() {
        let b = block(&cfg());
        let text = "# My Repo\n\nIntro.\n\n## Build\n\nmake\n";
        let once = inject(text, &b);
        assert!(once.starts_with("# My Repo\n\n<!-- whylog:start -->"));
        assert!(once.contains("Intro.\n\n## Build"));
        let twice = inject(&once, &b);
        assert_eq!(once, twice, "injecting again must be idempotent");
        assert_eq!(twice.matches(START).count(), 1);
        let back = remove(&twice);
        assert_eq!(back.matches(START).count(), 0);
        assert!(back.contains("Intro."));
    }

    #[test]
    fn empty_and_untitled_files() {
        let b = block(&cfg());
        assert!(inject("", &b).starts_with("# AGENTS.md\n\n<!-- whylog:start -->"));
        assert!(inject("no title here\n", &b).starts_with(START));
        assert!(imports_agents("@AGENTS.md\n"));
        assert!(imports_agents("Refer to `AGENTS.md` first.\n"));
        assert!(!imports_agents("# Notes\nuse tabs\n"));
    }

    #[test]
    fn block_has_no_em_dashes() {
        assert!(!block(&cfg()).contains('\u{2014}'));
        assert!(!BRIEFING.contains('\u{2014}'));
    }
}
