//! Majira (time): majira, sasa, sekunde, kutoka_sekunde, umbiza, lala.

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use std::collections::HashMap;

use crate::value::{self, Value};
use super::BuiltinFn;

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
            use std::time::{Duration, UNIX_EPOCH, SystemTime};
            let dur = Duration::from_secs_f64(secs);
            let datetime = UNIX_EPOCH + dur;
            let secs_total = dur.as_secs();
            let days_since_epoch = secs_total / 86400;
            let secs_today = secs_total % 86400;
            let hours = secs_today / 3600;
            let minutes = (secs_today % 3600) / 60;
            let seconds = secs_today % 60;
            let year = 1970 + (days_since_epoch / 365) as u32;
            let day_of_year = (days_since_epoch % 365) as u32;
            let month = ((day_of_year / 30).min(11)) + 1;
            let day = (day_of_year % 30) + 1;
            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hours, minutes, seconds)
        };
        #[cfg(target_arch = "wasm32")]
        let s = secs.to_string();
        Ok(Value::Neno(s))
    }));
    m.insert("lala".to_string(), Box::new(|args: &[Value]| {
        let secs = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0);
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::sleep(std::time::Duration::from_secs_f64(secs));
        }
        #[cfg(target_arch = "wasm32")]
        let _ = secs;
        Ok(Value::Tupu)
    }));
}
