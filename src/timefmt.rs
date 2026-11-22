//! UTC timestamps without a date-time dependency.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds and microseconds since the Unix epoch.
pub fn now() -> (u64, u32) {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (d.as_secs(), d.subsec_micros())
}

/// Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

struct Parts {
    y: i64,
    mo: u32,
    d: u32,
    h: u64,
    mi: u64,
    s: u64,
}

fn parts(secs: u64) -> Parts {
    let (y, mo, d) = civil_from_days((secs / 86_400) as i64);
    let rem = secs % 86_400;
    Parts {
        y,
        mo,
        d,
        h: rem / 3600,
        mi: (rem % 3600) / 60,
        s: rem % 60,
    }
}

/// `2026-11-22T18:40:12+00:00`
pub fn iso(secs: u64) -> String {
    let p = parts(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
        p.y, p.mo, p.d, p.h, p.mi, p.s
    )
}

/// `2026-11-22`
pub fn date(secs: u64) -> String {
    let p = parts(secs);
    format!("{:04}-{:02}-{:02}", p.y, p.mo, p.d)
}

/// Sortable id stem: `20261122T184012245181`
pub fn id_stem(secs: u64, micros: u32) -> String {
    let p = parts(secs);
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}{:06}",
        p.y, p.mo, p.d, p.h, p.mi, p.s, micros
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_instant() {
        // 2026-11-22T18:40:12Z
        let secs = 1_795_372_812;
        assert_eq!(iso(secs), "2026-11-22T18:40:12+00:00");
        assert_eq!(date(secs), "2026-11-22");
        assert_eq!(id_stem(secs, 245_181), "20261122T184012245181");
    }

    #[test]
    fn epoch_and_leap_day() {
        assert_eq!(iso(0), "1970-01-01T00:00:00+00:00");
        assert_eq!(date(951_782_400), "2000-02-29");
    }
}
