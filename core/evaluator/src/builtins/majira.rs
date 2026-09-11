//! Majira (time): majira, sasa, sekunde, kutoka_sekunde, umbiza, lala.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
use std::time::{SystemTime, UNIX_EPOCH};

use std::collections::HashMap;

use crate::value::{self, Value};
use super::BuiltinFn;

/// Convert days since the Unix epoch (1970-01-01) to a proleptic-Gregorian (year, month, day),
/// correctly handling leap years. Howard Hinnant's `civil_from_days` algorithm — pure integer
/// arithmetic, no external date/time crate needed. See
/// https://howardhinnant.github.io/date_algorithms.html#civil_from_days
#[cfg(not(target_arch = "wasm32"))]
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
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
    #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
    {
        0.0_f64
    }
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("majira".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Namba(now_secs()))
    }));
    m.insert("sasa".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Wakati(now_secs()))
    }));
    m.insert("sekunde".to_string(), Box::new(|args: &[Value]| {
        let secs = match args.first() {
            Some(Value::Wakati(s)) => *s,
            _ => value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0),
        };
        Ok(Value::Namba(secs))
    }));
    m.insert("kutoka_sekunde".to_string(), Box::new(|args: &[Value]| {
        let n = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0);
        Ok(Value::Wakati(n))
    }));
    m.insert("umbiza".to_string(), Box::new(|args: &[Value]| {
        let secs = match args.first() {
            Some(Value::Wakati(s)) => *s,
            _ => value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0),
        };
        #[cfg(not(target_arch = "wasm32"))]
        let s = {
            use std::time::Duration;
            let dur = Duration::from_secs_f64(secs);
            let secs_total = dur.as_secs();
            let days_since_epoch = (secs_total / 86400) as i64;
            let secs_today = secs_total % 86400;
            let hours = secs_today / 3600;
            let minutes = (secs_today % 3600) / 60;
            let seconds = secs_today % 60;
            let (year, month, day) = civil_from_days(days_since_epoch);
            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hours, minutes, seconds)
        };
        #[cfg(target_arch = "wasm32")]
        let s = secs.to_string();
        Ok(Value::Neno(s))
    }));
    m.insert("lala".to_string(), Box::new(|args: &[Value]| {
        let secs = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0);
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            std::thread::sleep(std::time::Duration::from_secs_f64(secs));
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        let _ = secs;
        Ok(Value::Tupu)
    }));
}
