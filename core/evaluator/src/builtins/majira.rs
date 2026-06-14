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
    // TODO: umbiza currently emits raw "seconds.millis" (e.g. "1718301234.567").
    // Should produce a human-readable datetime string like "2024-06-14 10:00:34" using
    // the `time` or `chrono` crate, or at minimum format as HH:MM:SS for elapsed durations.
    m.insert("umbiza".to_string(), Box::new(|args: &[Value]| {
        let secs = match args.first() {
            Some(Value::Wakati(s)) => *s,
            _ => value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0),
        };
        #[cfg(not(target_arch = "wasm32"))]
        let s = {
            use std::time::Duration;
            let d = Duration::from_secs_f64(secs);
            let secs_u = d.as_secs();
            let millis = d.subsec_millis();
            format!("{}.{:03}", secs_u, millis)
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
