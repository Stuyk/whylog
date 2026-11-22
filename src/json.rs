//! Minimal JSON output (no serde, keeps the binary dependency-free).

use crate::store::Session;

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn opt(s: &Option<String>) -> String {
    s.as_deref().map_or("null".into(), esc)
}

fn arr(v: &[String]) -> String {
    format!(
        "[{}]",
        v.iter().map(|s| esc(s)).collect::<Vec<_>>().join(",")
    )
}

pub fn session(s: &Session, path: &str) -> String {
    format!(
        "{{\"id\":{},\"short\":{},\"topic\":{},\"status\":{},\"opened\":{},\"closed\":{},\"summary\":{},\"decisions\":{},\"files\":{},\"path\":{}}}",
        esc(&s.id),
        esc(s.short()),
        opt(&s.topic),
        esc(&s.status),
        opt(&s.opened),
        opt(&s.closed),
        opt(&s.summary),
        arr(&s.decisions),
        arr(&s.files),
        esc(path)
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn escapes() {
        assert_eq!(super::esc("a\"b\\c\n\u{1}"), "\"a\\\"b\\\\c\\n\\u0001\"");
    }
}
