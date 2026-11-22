//! Compaction: turn closed walkthroughs into dated decision-log entries.
//!
//! Agent-driven in two steps:
//! 1. `whylog compact` prints a token-lean digest of closed sessions not yet compacted.
//! 2. The agent writes decision lines; `whylog compact --apply FILE --through ID` appends
//!    them under a dated heading, moves the marker, and prunes the compacted walkthroughs.

use crate::store::Session;

pub const MARKER_PREFIX: &str = "<!-- whylog:compacted-through ";
pub const MARKER_SUFFIX: &str = " -->";

pub fn decisions_skeleton(project: &str) -> String {
    format!(
        "# {project} - Decision Log\n\n\
         Lasting decisions and why they were made. Read the section for the area you are\n\
         changing before you change established behaviour. Format: **what** - *why*.\n\
         Maintained by `whylog`: sessions record decisions, `whylog compact` folds them in here.\n"
    )
}

/// Id of the newest session already compacted, if any.
pub fn marker(decisions: &str) -> Option<String> {
    decisions.lines().find_map(|l| {
        l.trim()
            .strip_prefix(MARKER_PREFIX)
            .and_then(|r| r.strip_suffix(MARKER_SUFFIX))
            .map(|s| s.trim().to_string())
    })
}

pub fn set_marker(decisions: &str, through: &str) -> String {
    let line = format!("{MARKER_PREFIX}{through}{MARKER_SUFFIX}");
    if marker(decisions).is_some() {
        decisions
            .lines()
            .map(|l| {
                if l.trim().starts_with(MARKER_PREFIX) {
                    line.as_str()
                } else {
                    l
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    } else {
        // after the title line
        let mut lines: Vec<&str> = decisions.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.starts_with("# "))
            .map_or(0, |i| i + 1);
        lines.insert(at, &line);
        lines.join("\n") + "\n"
    }
}

/// Closed sessions newer than the marker, oldest first.
pub fn pending<'a>(sessions: &'a [Session], marker: Option<&str>) -> Vec<&'a Session> {
    let mut v: Vec<&Session> = sessions
        .iter()
        .filter(|s| !s.is_open())
        .filter(|s| marker.is_none_or(|m| s.id.as_str() > m))
        .collect();
    v.sort_by(|a, b| a.id.cmp(&b.id));
    v
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", cut.trim_end())
    }
}

/// Plain-text digest designed to be cheap to read.
pub fn digest(pending: &[&Session], max_note_chars: usize) -> String {
    let mut out = String::new();
    let Some(last) = pending.last() else {
        return "whylog: nothing to compact (no closed sessions since the last compaction)\n"
            .into();
    };
    let first = pending[0];
    out.push_str(&format!(
        "# whylog compact: {} sessions ({} → {})\n",
        pending.len(),
        &first.id[..8.min(first.id.len())],
        &last.id[..8.min(last.id.len())]
    ));
    out.push_str(
        "# Turn this into lasting decisions: one markdown bullet per line, `- **what** - *why*`.\n\
         # Keep conventions, removals, reverts and \"we do X, not Y\"; drop one-off tuning.\n\
         # Prefix a line with `[Area]` to group it (e.g. `[API] - **…** - *…*`).\n\
         # Style (`whylog style`): short, direct, active voice, real numbers, no hedging, no em-dashes.\n",
    );
    out.push_str(&format!(
        "# Then run: whylog compact --apply <file|-> --through {}\n\n",
        last.id
    ));
    for s in pending {
        let date = s.id.get(..8).unwrap_or(&s.id);
        out.push_str(&format!(
            "## {date} {} [{}]\n",
            s.topic.as_deref().unwrap_or("-"),
            s.short()
        ));
        if let Some(sum) = &s.summary {
            out.push_str(&format!("S: {}\n", squash(sum)));
        }
        for d in &s.decisions {
            out.push_str(&format!("D: {}\n", squash(d)));
        }
        if max_note_chars > 0 {
            let notes = squash(
                &s.notes
                    .iter()
                    .map(|l| l.trim_start().strip_prefix("- ").unwrap_or(l.trim_start()))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            if !notes.is_empty() {
                out.push_str(&format!("N: {}\n", clip(&notes, max_note_chars)));
            }
        }
        out.push('\n');
    }
    out
}

/// One parsed line of agent output: optional `[Area]` group plus a normalized bullet.
pub fn parse_lines(input: &str) -> Vec<(Option<String>, String)> {
    input
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (area, rest) = match l.strip_prefix('[').and_then(|r| r.split_once(']')) {
                Some((a, r)) if !a.trim().is_empty() => (Some(a.trim().to_string()), r.trim()),
                _ => (None, l),
            };
            let rest = rest
                .strip_prefix("- ")
                .or_else(|| rest.strip_prefix("* "))
                .unwrap_or(rest)
                .trim();
            (area, format!("- {rest}"))
        })
        .filter(|(_, l)| l.len() > 2)
        .collect()
}

/// Append a dated compaction section. Lines with an area become `### Area` subsections.
pub fn append_entry(
    decisions: &str,
    date: &str,
    count: usize,
    lines: &[(Option<String>, String)],
) -> String {
    let mut out = decisions.trim_end().to_string();
    out.push_str(&format!(
        "\n\n## Compaction {date} ({count} session{})\n",
        if count == 1 { "" } else { "s" }
    ));
    let mut areas: Vec<Option<&String>> = Vec::new();
    for (a, _) in lines {
        if !areas.contains(&a.as_ref()) {
            areas.push(a.as_ref());
        }
    }
    // ungrouped first
    areas.sort_by_key(|a| a.is_some());
    for area in areas {
        out.push('\n');
        if let Some(a) = area {
            out.push_str(&format!("### {a}\n\n"));
        }
        for (_, l) in lines.iter().filter(|(x, _)| x.as_ref() == area) {
            out.push_str(l);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn s(id: &str, open: bool) -> Session {
        Session {
            id: id.into(),
            status: if open { "open".into() } else { "closed".into() },
            summary: Some("did things".into()),
            decisions: vec!["**x** - *y*".into()],
            notes: vec!["- note one".into(), "- note two".into()],
            path: PathBuf::new(),
            ..Default::default()
        }
    }

    #[test]
    fn marker_roundtrip() {
        let d = decisions_skeleton("P");
        assert!(marker(&d).is_none());
        let d = set_marker(&d, "20261122T1-aaaa");
        assert_eq!(marker(&d).as_deref(), Some("20261122T1-aaaa"));
        let d = set_marker(&d, "20261123T1-bbbb");
        assert_eq!(marker(&d).as_deref(), Some("20261123T1-bbbb"));
        assert_eq!(d.matches(MARKER_PREFIX).count(), 1);
    }

    #[test]
    fn pending_skips_open_and_compacted() {
        let all = vec![
            s("20261120T1-aaaa", false),
            s("20261121T1-bbbb", true),
            s("20261122T1-cccc", false),
        ];
        let p = pending(&all, Some("20261120T1-aaaa"));
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].id, "20261122T1-cccc");
        let dg = digest(&p, 100);
        assert!(dg.contains("--through 20261122T1-cccc"));
        assert!(dg.contains("D: **x** - *y*"));
        assert!(dg.contains("N: note one note two"));
    }

    #[test]
    fn parse_and_append() {
        let lines =
            parse_lines("# comment\n[Audio] - **a** - *b*\n* **c** - *d*\n\n[Audio] **e** - *f*\n");
        assert_eq!(lines.len(), 3);
        let out = append_entry("# T\n", "2026-11-22", 2, &lines);
        assert!(out.contains("## Compaction 2026-11-22 (2 sessions)"));
        assert!(out.contains("- **c** - *d*\n\n### Audio\n\n- **a** - *b*\n- **e** - *f*\n"));
    }
}
