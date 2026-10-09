//! Majira (time): majira, sasa, sekunde, kutoka_sekunde, umbiza, lala.

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use std::collections::HashMap;

use super::BuiltinFn;
use crate::value::{self, EvalError, Value};

/// Convert days since the Unix epoch (1970-01-01) to a proleptic-Gregorian (year, month, day),
/// correctly handling leap years. Howard Hinnant's `civil_from_days` algorithm — pure integer
/// arithmetic, no external date/time crate needed. See
/// https://howardhinnant.github.io/date_algorithms.html#civil_from_days
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn now_secs() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
    #[cfg(target_arch = "wasm32")]
    {
        0.0_f64
    }
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "majira".to_string(),
        Box::new(|_args: &[Value]| Ok(Value::Namba(now_secs()))),
    );
    m.insert(
        "sasa".to_string(),
        Box::new(|_args: &[Value]| Ok(Value::Wakati(now_secs()))),
    );
    m.insert(
        "sekunde".to_string(),
        Box::new(|args: &[Value]| {
            let secs = match args.first() {
                Some(Value::Wakati(s)) => *s,
                _ => value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0),
            };
            Ok(Value::Namba(secs))
        }),
    );
    m.insert(
        "kutoka_sekunde".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0);
            Ok(Value::Wakati(n))
        }),
    );
    m.insert(
        "umbiza".to_string(),
        Box::new(|args: &[Value]| {
            let secs = wakati_arg(args);
            Ok(Value::neno(match Parts::of(secs) {
                Some(p) => p.text(' ', false, None),
                None => namba_text(secs),
            }))
        }),
    );
    m.insert(
        "kwa_iso".to_string(),
        Box::new(|args: &[Value]| {
            let secs = wakati_arg(args);
            Ok(Value::neno(match Parts::of(secs) {
                Some(p) => p.text('T', true, Some(0)),
                None => namba_text(secs),
            }))
        }),
    );
    m.insert(
        "umbiza_eneo".to_string(),
        Box::new(|args: &[Value]| {
            let secs = wakati_arg(args);
            let offset = value::as_f64(args.get(1).unwrap_or(&Value::Hamna))
                .filter(|m| m.is_finite() && m.abs() <= 18.0 * 60.0)
                .ok_or_else(|| {
                    EvalError::TypeErr(
                        "umbiza_eneo inahitaji tofauti ya dakika kati ya -1080 na 1080".into(),
                    )
                })?;
            let minutes = offset.round() as i64;
            Ok(Value::neno(match Parts::of(secs + 60.0 * minutes as f64) {
                Some(p) => p.text(' ', false, Some(minutes)),
                None => namba_text(secs),
            }))
        }),
    );
    m.insert(
        "kutoka_iso".to_string(),
        Box::new(|args: &[Value]| {
            let text = super::arg_str(args, 0);
            Ok(match parse_iso(text.trim()) {
                Some(secs) => Value::sawa(Value::Wakati(secs)),
                None => Value::kosa(format!("'{text}' si tarehe ya ISO 8601")),
            })
        }),
    );
    m.insert(
        "kutoka_tarehe".to_string(),
        Box::new(|args: &[Value]| {
            let n = |i: usize| value::as_f64(args.get(i).unwrap_or(&Value::Hamna));
            let (Some(y), Some(mo), Some(d)) = (n(0), n(1), n(2)) else {
                return Err(EvalError::TypeErr(
                    "kutoka_tarehe inahitaji mwaka, mwezi na siku".into(),
                ));
            };
            Ok(match valid_date(y, mo, d) {
                Some((y, mo, d)) => {
                    Value::sawa(Value::Wakati(days_from_civil(y, mo, d) as f64 * 86_400.0))
                }
                None => Value::kosa(format!("{y}-{mo}-{d} si tarehe halali")),
            })
        }),
    );
    m.insert(
        "tarehe".to_string(),
        Box::new(|args: &[Value]| {
            let secs = wakati_arg(args);
            let p = Parts::of(secs)
                .ok_or_else(|| EvalError::TypeErr("tarehe inahitaji wakati wenye mwisho".into()))?;
            let mut m = crate::value::Kamusi::default();
            for (k, v) in [
                ("mwaka", p.year as f64),
                ("mwezi", p.month as f64),
                ("siku", p.day as f64),
                ("saa", p.hour as f64),
                ("dakika", p.minute as f64),
                ("sekunde", p.second as f64 + p.fraction),
                // 1 = Jumatatu (Monday) … 7 = Jumapili (Sunday), as ISO 8601 numbers them.
                ("siku_ya_wiki", p.weekday as f64),
                ("siku_ya_mwaka", p.day_of_year as f64),
            ] {
                m.insert(crate::value::MapKey::Neno(k.into()), Value::Namba(v));
            }
            Ok(Value::Kamusi(std::rc::Rc::new(m)))
        }),
    );
    m.insert(
        "kipima_muda".to_string(),
        Box::new(|_args: &[Value]| Ok(Value::Namba(monotonic_secs()))),
    );
    m.insert(
        "lala".to_string(),
        Box::new(|args: &[Value]| {
            let secs = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0);
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::platform::flush_stdout();
                std::thread::sleep(std::time::Duration::from_secs_f64(secs));
            }
            #[cfg(target_arch = "wasm32")]
            let _ = secs;
            Ok(Value::Tupu)
        }),
    );
}

/// Seconds since the epoch from a `Wakati` (or a number) argument.
fn wakati_arg(args: &[Value]) -> f64 {
    match args.first() {
        Some(Value::Wakati(s)) => *s,
        other => value::as_f64(other.unwrap_or(&Value::Hamna)).unwrap_or(0.0),
    }
}

fn namba_text(n: f64) -> String {
    value::namba_text(n).to_string()
}

/// Days since 1970-01-01 of a proleptic-Gregorian date (inverse of [`civil_from_days`]).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

fn leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if leap(y) => 29,
        _ => 28,
    }
}

/// `(y, m, d)` when they name a real date (whole numbers, month 1–12, day within the month).
fn valid_date(y: f64, m: f64, d: f64) -> Option<(i64, u32, u32)> {
    let whole = |x: f64| x.is_finite() && x.fract() == 0.0;
    if !(whole(y) && whole(m) && whole(d)) || y.abs() > 1.0e6 || !(1.0..=12.0).contains(&m) {
        return None;
    }
    let (y, m) = (y as i64, m as u32);
    (d >= 1.0 && d <= days_in_month(y, m) as f64).then_some((y, m, d as u32))
}

/// A moment's calendar fields (UTC).
struct Parts {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    fraction: f64,
    weekday: u32,
    day_of_year: u32,
}

impl Parts {
    /// The fields of `secs` since the epoch; `None` when it is not finite or beyond ±a million
    /// years.
    fn of(secs: f64) -> Option<Parts> {
        if !secs.is_finite() || secs.abs() > 3.0e13 {
            return None;
        }
        let whole = secs.floor();
        let fraction = secs - whole;
        let whole = whole as i64;
        let days = whole.div_euclid(86_400);
        let rest = whole.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Some(Parts {
            year,
            month,
            day,
            hour: (rest / 3600) as u32,
            minute: (rest % 3600 / 60) as u32,
            second: (rest % 60) as u32,
            fraction,
            // 1970-01-01 was a Thursday (4).
            weekday: ((days + 3).rem_euclid(7) + 1) as u32,
            day_of_year: (days - days_from_civil(year, 1, 1)) as u32 + 1,
        })
    }

    /// `YYYY-MM-DD<sep>HH:MM:SS`, with milliseconds when there is a fraction and `millis`
    /// asks for them, and the UTC offset when given (`Z` for 0 in ISO form).
    fn text(&self, sep: char, iso: bool, offset_minutes: Option<i64>) -> String {
        let mut s = format!(
            "{:04}-{:02}-{:02}{sep}{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        );
        if iso {
            let ms = (self.fraction * 1000.0).floor() as u32;
            if ms > 0 {
                s.push_str(&format!(".{ms:03}"));
            }
        }
        match offset_minutes {
            Some(0) if iso => s.push('Z'),
            Some(m) => {
                let sign = if m < 0 { '-' } else { '+' };
                s.push_str(&format!("{sign}{:02}:{:02}", m.abs() / 60, m.abs() % 60));
            }
            None => {}
        }
        s
    }
}

/// Seconds since the epoch of an ISO 8601 date or date-time: `YYYY-MM-DD`, optionally
/// `THH:MM[:SS[.fraction]]` (or a space for `T`), optionally `Z` or `±HH:MM` / `±HHMM`
/// (no offset: UTC).
fn parse_iso(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let num = |from: usize, len: usize| -> Option<i64> {
        let part = s.get(from..from + len)?;
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, mo, d) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (y, mo, d) = valid_date(y as f64, mo as f64, d as f64)?;
    let mut secs = days_from_civil(y, mo, d) as f64 * 86_400.0;
    let mut i = 10;
    if i < b.len() && (b[i] == b'T' || b[i] == b't' || b[i] == b' ') {
        let (h, mi) = (num(i + 1, 2)?, num(i + 4, 2)?);
        if b.get(i + 3) != Some(&b':') || h > 23 || mi > 59 {
            return None;
        }
        secs += (h * 3600 + mi * 60) as f64;
        i += 6;
        if b.get(i) == Some(&b':') {
            let sec = num(i + 1, 2)?;
            if sec > 60 {
                return None;
            }
            secs += sec as f64;
            i += 3;
            if b.get(i) == Some(&b'.') || b.get(i) == Some(&b',') {
                let start = i + 1;
                let mut end = start;
                while end < b.len() && b[end].is_ascii_digit() {
                    end += 1;
                }
                if end == start {
                    return None;
                }
                secs += format!("0.{}", &s[start..end]).parse::<f64>().ok()?;
                i = end;
            }
        }
    }
    match b.get(i) {
        None => Some(secs),
        Some(b'Z' | b'z') if i + 1 == b.len() => Some(secs),
        Some(&sign @ (b'+' | b'-')) => {
            let h = num(i + 1, 2)?;
            let m = match (b.get(i + 3), b.len() - i) {
                (Some(b':'), 6) => num(i + 4, 2)?,
                (_, 5) => num(i + 3, 2)?,
                (_, 3) => 0,
                _ => return None,
            };
            if h > 23 || m > 59 {
                return None;
            }
            let offset = (h * 3600 + m * 60) as f64;
            // Local time = UTC + offset, so UTC = local − offset.
            Some(if sign == b'+' {
                secs - offset
            } else {
                secs + offset
            })
        }
        _ => None,
    }
}

/// Seconds on a clock that only moves forward, from the first call: for measuring how long
/// something takes (`sasa` follows the wall clock, which can jump).
fn monotonic_secs() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
    }
    #[cfg(target_arch = "wasm32")]
    {
        now_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip() {
        for days in [-800_000i64, -1, 0, 1, 59, 60, 10_957, 19_000, 2_932_896] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(
            Parts::of(0.0).unwrap().text('T', true, Some(0)),
            "1970-01-01T00:00:00Z"
        );
        assert_eq!(
            Parts::of(-1.5).unwrap().text('T', true, Some(0)),
            "1969-12-31T23:59:58.500Z"
        );
        assert_eq!(Parts::of(951_782_400.0).unwrap().day, 29); // 2000-02-29
        assert_eq!(Parts::of(0.0).unwrap().weekday, 4); // Thursday
    }

    #[test]
    fn iso_parsing() {
        assert_eq!(parse_iso("1970-01-01"), Some(0.0));
        assert_eq!(parse_iso("1970-01-01T00:00:01Z"), Some(1.0));
        assert_eq!(parse_iso("1970-01-01T03:00:00+03:00"), Some(0.0));
        assert_eq!(parse_iso("1970-01-01 00:00:00-0130"), Some(5400.0));
        assert_eq!(parse_iso("2024-02-29T12:30:15.25Z"), Some(1_709_209_815.25));
        for bad in [
            "2023-02-29",
            "2024-13-01",
            "2024-01-01T24:00",
            "x",
            "2024-01-01Zx",
            "2024-1-01",
        ] {
            assert_eq!(parse_iso(bad), None, "{bad}");
        }
    }
}
