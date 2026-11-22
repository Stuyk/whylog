//! Project discovery and `.whylog.toml`.
//!
//! The config is a flat `key = "value"` file at the project root:
//!
//! ```toml
//! sessions_dir = "docs/walkthroughs"
//! decisions_file = "docs/decisions.md"
//! ```

use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = ".whylog.toml";
pub const DEFAULT_SESSIONS_DIR: &str = ".whylog/sessions";
pub const DEFAULT_DECISIONS_FILE: &str = "DECISIONS.md";
pub const DEFAULT_COMPACT_AFTER: usize = 25;

#[derive(Debug, Clone)]
pub struct Config {
    pub root: PathBuf,
    pub sessions_dir: PathBuf,
    pub decisions_file: PathBuf,
    /// Optional project-specific writing rules, appended to `whylog style`.
    pub style_file: Option<PathBuf>,
    /// Recommend compaction once this many closed sessions are waiting (0 = never).
    pub compact_after: usize,
    /// True when a `.whylog.toml` was found (vs. defaults).
    pub from_file: bool,
}

impl Config {
    /// Path relative to the project root, for display.
    pub fn rel(&self, p: &Path) -> String {
        p.strip_prefix(&self.root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

/// Walk up from `start` to the nearest `.whylog.toml`; fall back to the git root, then `start`.
pub fn discover(start: &Path) -> Config {
    let mut git_root = None;
    let mut dir = Some(start);
    while let Some(d) = dir {
        let cfg = d.join(CONFIG_FILE);
        if cfg.is_file() {
            let text = std::fs::read_to_string(&cfg).unwrap_or_default();
            return from_text(d, &text, true);
        }
        if git_root.is_none() && d.join(".git").exists() {
            git_root = Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    let root = git_root.unwrap_or_else(|| start.to_path_buf());
    from_text(&root, "", false)
}

pub fn from_text(root: &Path, text: &str, from_file: bool) -> Config {
    let mut sessions = DEFAULT_SESSIONS_DIR.to_string();
    let mut decisions = DEFAULT_DECISIONS_FILE.to_string();
    let mut style = None;
    let mut compact_after = DEFAULT_COMPACT_AFTER;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
        match k.trim() {
            "sessions_dir" => sessions = v,
            "decisions_file" => decisions = v,
            "style_file" => style = Some(v),
            "compact_after" => compact_after = v.parse().unwrap_or(DEFAULT_COMPACT_AFTER),
            _ => {}
        }
    }
    Config {
        root: root.to_path_buf(),
        sessions_dir: root.join(sessions),
        decisions_file: root.join(decisions),
        compact_after,
        style_file: style.filter(|s| !s.is_empty()).map(|s| root.join(s)),
        from_file,
    }
}

pub fn render(sessions_dir: &str, decisions_file: &str) -> String {
    format!(
        "# whylog project config. Paths are relative to this file.\n\
         sessions_dir = \"{sessions_dir}\"\n\
         decisions_file = \"{decisions_file}\"\n\
         # style_file = \"docs/writing-style.md\"  # optional extra writing rules for `whylog style`\n\
         # compact_after = 25                     # recommend `whylog compact` after this many closed sessions (0 = off)\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keys_and_defaults() {
        let root = Path::new("/p");
        let c = from_text(root, "# c\nsessions_dir = \"docs/w\"\n", true);
        assert_eq!(c.sessions_dir, Path::new("/p/docs/w"));
        assert_eq!(c.decisions_file, Path::new("/p/DECISIONS.md"));
        assert_eq!(c.rel(&c.sessions_dir), "docs/w");
        assert_eq!(c.compact_after, 25);
        let c = from_text(root, "compact_after = 10\n", true);
        assert_eq!(c.compact_after, 10);
    }
}
