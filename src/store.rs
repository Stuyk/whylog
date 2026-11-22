//! Walkthrough files: one markdown file per session, no separate registry.
//!
//! ```text
//! # Session <id>
//!
//! - Topic: <topic>
//! - Opened: <iso>
//! - Closed: <iso>          (added on close)
//! - Status: open|closed
//!
//! ## Summary
//! ## Decisions
//! ## Notes
//! ## Files Modified
//! ```

use crate::config::Config;
use crate::timefmt;
use std::collections::hash_map::RandomState;
use std::fs::{self, File};
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::path::{Path, PathBuf};

pub const PENDING: &str = "_pending_";
pub const NONE_MARK: &str = "_none_";
pub const SUMMARY: &str = "## Summary";
pub const DECISIONS: &str = "## Decisions";
pub const NOTES: &str = "## Notes";
pub const FILES: &str = "## Files Modified";

#[derive(Debug, Clone, Default)]
pub struct Session {
    pub id: String,
    pub topic: Option<String>,
    pub opened: Option<String>,
    pub closed: Option<String>,
    pub status: String,
    pub summary: Option<String>,
    pub decisions: Vec<String>,
    pub notes: Vec<String>,
    pub files: Vec<String>,
    pub path: PathBuf,
}

impl Session {
    pub fn is_open(&self) -> bool {
        self.status == "open"
    }
    /// The 4-hex random suffix, handy as a short handle.
    pub fn short(&self) -> &str {
        self.id.rsplit('-').next().unwrap_or(&self.id)
    }
}

pub fn new_id() -> String {
    let (secs, micros) = timefmt::now();
    let mut h = RandomState::new().build_hasher();
    h.write_u32(micros);
    h.write_u32(std::process::id());
    format!(
        "{}-{:04x}",
        timefmt::id_stem(secs, micros),
        h.finish() as u16
    )
}

pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    let out = out.trim_end_matches('-');
    let out: String = out.chars().take(60).collect();
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "general".into()
    } else {
        out
    }
}

pub fn skeleton(id: &str, topic: &str, opened: &str) -> String {
    let topic = if topic.trim().is_empty() {
        "(none)"
    } else {
        topic.trim()
    };
    format!(
        "# Session {id}\n\n- Topic: {topic}\n- Opened: {opened}\n- Status: open\n\n\
         {SUMMARY}\n\n{PENDING}\n\n{DECISIONS}\n\n{NOTES}\n\n{FILES}\n\n{NONE_MARK}\n"
    )
}

// ---------------------------------------------------------------- sections

fn header_value(lines: &[&str], key: &str) -> Option<String> {
    let prefix = format!("- {key}:");
    lines
        .iter()
        .take_while(|l| !l.starts_with("## "))
        .find_map(|l| l.strip_prefix(&prefix).map(|v| v.trim().to_string()))
}

/// Body lines of `heading` (blank lines trimmed at both ends).
pub fn section<'a>(text: &'a str, heading: &str) -> Option<Vec<&'a str>> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| l.trim_end() == heading)? + 1;
    let end = lines[start..]
        .iter()
        .position(|l| l.starts_with("## "))
        .map_or(lines.len(), |i| start + i);
    let mut body = lines[start..end].to_vec();
    while body.first().is_some_and(|l| l.trim().is_empty()) {
        body.remove(0);
    }
    while body.last().is_some_and(|l| l.trim().is_empty()) {
        body.pop();
    }
    Some(body)
}

fn bullets(body: &[&str]) -> Vec<String> {
    body.iter()
        .filter_map(|l| l.strip_prefix("- "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != NONE_MARK)
        .collect()
}

/// Replace the body of `heading`, creating the section (before `## Files Modified`,
/// else at the end) when it is missing.
pub fn set_section(text: &str, heading: &str, body: &[String]) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let block: Vec<String> = std::iter::once(String::new())
        .chain(body.iter().cloned())
        .chain(std::iter::once(String::new()))
        .collect();
    if let Some(pos) = lines.iter().position(|l| l.trim_end() == heading) {
        let end = lines[pos + 1..]
            .iter()
            .position(|l| l.starts_with("## "))
            .map_or(lines.len(), |i| pos + 1 + i);
        lines.splice(pos + 1..end, block);
    } else {
        let insert_at = if heading != FILES {
            lines.iter().position(|l| l.trim_end() == FILES)
        } else {
            None
        };
        let mut new_block = vec![heading.to_string()];
        new_block.extend(block);
        match insert_at {
            Some(i) => {
                lines.splice(i..i, new_block);
            }
            None => {
                while lines.last().is_some_and(|l| l.trim().is_empty()) {
                    lines.pop();
                }
                lines.push(String::new());
                lines.extend(new_block);
            }
        }
    }
    let mut out = lines.join("\n");
    while out.ends_with("\n\n") || out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    // collapse runs of 3+ newlines produced by splicing
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out
}

/// Append raw lines to the end of `heading`'s body.
pub fn append_section(text: &str, heading: &str, extra: &[String]) -> String {
    let mut body: Vec<String> = section(text, heading)
        .unwrap_or_default()
        .iter()
        .map(|s| s.to_string())
        .filter(|s| s.trim() != NONE_MARK)
        .collect();
    body.extend(extra.iter().cloned());
    set_section(text, heading, &body)
}

pub fn parse_text(text: &str, path: &Path) -> Session {
    let lines: Vec<&str> = text.lines().collect();
    let id = lines
        .first()
        .and_then(|l| l.strip_prefix("# Session "))
        .map(|s| s.trim().to_string())
        .or_else(|| id_from_filename(path))
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        });
    let topic = header_value(&lines, "Topic").filter(|t| t != "(none)" && !t.is_empty());
    let summary = section(text, SUMMARY)
        .map(|b| b.join("\n").trim().to_string())
        .filter(|s| !s.is_empty() && s != PENDING);
    let notes: Vec<String> = section(text, NOTES)
        .unwrap_or_default()
        .iter()
        .map(|s| s.to_string())
        .collect();
    Session {
        id,
        topic,
        opened: header_value(&lines, "Opened"),
        closed: header_value(&lines, "Closed"),
        status: header_value(&lines, "Status").unwrap_or_else(|| "unknown".into()),
        summary,
        decisions: bullets(&section(text, DECISIONS).unwrap_or_default()),
        notes,
        files: bullets(&section(text, FILES).unwrap_or_default()),
        path: path.to_path_buf(),
    }
}

/// `session-<stem>-<hex>-<slug>.md` → `<stem>-<hex>`
pub fn id_from_filename(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let rest = name.strip_prefix("session-")?.strip_suffix(".md")?;
    let mut parts = rest.splitn(3, '-');
    let stem = parts.next()?;
    let hex = parts.next()?;
    Some(format!("{stem}-{hex}"))
}

// -------------------------------------------------------------------- files

pub fn session_files(cfg: &Config) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(&cfg.sessions_dir) else {
        return vec![];
    };
    let mut v: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|e| e == "md")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("session-"))
        })
        .collect();
    v.sort();
    v
}

pub fn load(path: &Path) -> io::Result<Session> {
    Ok(parse_text(&fs::read_to_string(path)?, path))
}

pub fn all(cfg: &Config) -> Vec<Session> {
    session_files(cfg)
        .iter()
        .filter_map(|p| load(p).ok())
        .collect()
}

/// Resolve a full id, the 4-hex suffix, or any unique substring of the id.
pub fn resolve(cfg: &Config, query: &str) -> Result<Session, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("empty session id".into());
    }
    let files = session_files(cfg);
    let ids: Vec<(String, &PathBuf)> = files
        .iter()
        .filter_map(|p| id_from_filename(p).map(|id| (id, p)))
        .collect();
    if let Some((_, p)) = ids.iter().find(|(id, _)| id == query) {
        return load(p).map_err(|e| e.to_string());
    }
    // A 4-hex handle wins over substring matches (timestamps can contain the same digits).
    let suffix = format!("-{query}");
    let mut hits: Vec<&(String, &PathBuf)> =
        ids.iter().filter(|(id, _)| id.ends_with(&suffix)).collect();
    if hits.is_empty() {
        hits = ids.iter().filter(|(id, _)| id.contains(query)).collect();
    }
    match hits.as_slice() {
        [one] => load(one.1).map_err(|e| e.to_string()),
        [] => Err(format!("no session matches '{query}'")),
        many => Err(format!(
            "'{query}' matches {} sessions; use more of the id ({})",
            many.len(),
            many.iter()
                .take(3)
                .map(|h| h.0.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

pub fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("md.whylog-tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

/// Exclusive cross-process lock for read-modify-write. Lives in the temp dir so it never
/// shows up in the project.
pub struct Lock(#[allow(dead_code)] File);

pub fn lock(cfg: &Config) -> io::Result<Lock> {
    let mut h = std::hash::DefaultHasher::new();
    h.write(cfg.root.to_string_lossy().as_bytes());
    let path = std::env::temp_dir().join(format!("whylog-{:016x}.lock", h.finish()));
    let f = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    f.lock()?;
    Ok(Lock(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_rules() {
        assert_eq!(
            slugify("Fix bravo_02 Glass again!"),
            "fix-bravo-02-glass-again"
        );
        assert_eq!(slugify("  "), "general");
    }

    #[test]
    fn skeleton_roundtrip() {
        let t = skeleton(
            "20261122T000000000000-ab12",
            "gibs",
            "2026-11-22T00:00:00+00:00",
        );
        let s = parse_text(&t, Path::new("session-20261122T000000000000-ab12-gibs.md"));
        assert_eq!(s.id, "20261122T000000000000-ab12");
        assert_eq!(s.topic.as_deref(), Some("gibs"));
        assert!(s.is_open());
        assert!(s.summary.is_none());
        assert!(s.files.is_empty() && s.decisions.is_empty());
        assert_eq!(s.short(), "ab12");
    }

    #[test]
    fn append_and_set_sections() {
        let t = skeleton("x-0000", "t", "o");
        let t = append_section(&t, NOTES, &["- first".into()]);
        let t = append_section(&t, NOTES, &["- second".into()]);
        let t = append_section(&t, DECISIONS, &["- **a** - *b*".into()]);
        let t = set_section(&t, FILES, &["- a.gd".into(), "- b.gd".into()]);
        let s = parse_text(&t, Path::new("session-x-0000-t.md"));
        assert_eq!(s.notes, vec!["- first", "- second"]);
        assert_eq!(s.decisions, vec!["**a** - *b*"]);
        assert_eq!(s.files, vec!["a.gd", "b.gd"]);
        assert!(!t.contains("\n\n\n"));
    }

    #[test]
    fn legacy_file_without_decisions_section() {
        let t = "# Session 1-ab\n\n- Topic: old\n- Opened: o\n- Status: open\n\n## Summary\n\n_pending_\n\n## Notes\n\n";
        let t2 = append_section(t, DECISIONS, &["- d".into()]);
        assert!(t2.contains("## Decisions\n\n- d"));
        let s = parse_text(&t2, Path::new("session-1-ab-old.md"));
        assert_eq!(s.decisions, vec!["d"]);
    }

    #[test]
    fn handle_beats_substring() {
        let dir = std::env::temp_dir().join(format!("whylog-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cfg = crate::config::from_text(&dir, "", false);
        for id in ["20261122T190542554753-16b9", "20261122T190542560086-4753"] {
            write_atomic(
                &cfg.sessions_dir.join(format!("session-{id}-t.md")),
                &skeleton(id, "t", "o"),
            )
            .unwrap();
        }
        assert_eq!(
            resolve(&cfg, "4753").unwrap().id,
            "20261122T190542560086-4753"
        );
        assert_eq!(
            resolve(&cfg, "16b9").unwrap().id,
            "20261122T190542554753-16b9"
        );
        assert!(resolve(&cfg, "20261122T1905425").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn filename_ids() {
        assert_eq!(
            id_from_filename(Path::new("session-20261122T1-be5b-fix-editor-warnings.md"))
                .as_deref(),
            Some("20261122T1-be5b")
        );
    }
}
