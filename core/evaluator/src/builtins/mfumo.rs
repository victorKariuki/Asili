//! Mfumo (system): vigezo, pata_env, toka, sikiliza_ishara, rejesha_ishara.

use std::collections::HashMap;

use super::BuiltinFn;
use crate::signal;
use crate::value::{self, Value};

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "vigezo".to_string(),
        Box::new(|_args: &[Value]| {
            #[cfg(not(target_arch = "wasm32"))]
            let args_vec: Vec<Value> = std::env::args().map(Value::neno).collect();
            #[cfg(target_arch = "wasm32")]
            let args_vec: Vec<Value> = Vec::new();
            Ok(Value::list(args_vec))
        }),
    );
    m.insert(
        "pata_env".to_string(),
        Box::new(|args: &[Value]| {
            #[cfg(not(target_arch = "wasm32"))]
            let val = {
                let name = super::arg_str(args, 0);
                std::env::var(&name).ok().map(|s| Box::new(Value::neno(s)))
            };
            #[cfg(target_arch = "wasm32")]
            let val = None::<Box<Value>>;
            Ok(Value::Chaguo(val))
        }),
    );
    // The watchdog: once armed, the program must feed it (`mlinzi_lisha`) within `ms`
    // milliseconds every time, or it enters its safe state and stops (exit code 5).
    m.insert(
        "mlinzi_anza".to_string(),
        Box::new(|args: &[Value]| {
            let ms = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0);
            if !(ms.is_finite() && ms > 0.0) {
                return Ok(Value::kosa("mlinzi_anza inahitaji muda chanya (ms)"));
            }
            mlinzi::anza(std::time::Duration::from_secs_f64(ms / 1000.0));
            Ok(Value::sawa(Value::Tupu))
        }),
    );
    m.insert(
        "mlinzi_lisha".to_string(),
        Box::new(|_args: &[Value]| {
            mlinzi::lisha();
            Ok(Value::Tupu)
        }),
    );
    // The memory limit, in bytes (0: none): passing it stops the program through its safe state.
    m.insert(
        "kikomo_kumbukumbu".to_string(),
        Box::new(|args: &[Value]| {
            let bytes = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0);
            crate::alloc::set_limit(bytes.max(0.0) as usize);
            Ok(Value::Tupu)
        }),
    );
    m.insert(
        "toka".to_string(),
        Box::new(|args: &[Value]| {
            let code = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0) as i32;
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::platform::flush_stdout();
                asili_trace::finish();
                std::process::exit(code);
            }
            #[cfg(target_arch = "wasm32")]
            {
                let _ = code;
                Ok(Value::Tupu)
            }
        }),
    );
    m.insert(
        "sikiliza_ishara".to_string(),
        Box::new(|args: &[Value]| {
            #[cfg(unix)]
            {
                let sig_id =
                    value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0) as i32;
                let kazi_name = super::arg_str(args, 1);
                signal::register_handler(sig_id, kazi_name);
                Ok(Value::sawa(Value::Tupu))
            }
            #[cfg(not(unix))]
            {
                Ok(Value::kosa(
                    "sikiliza_ishara: sifa haipo kwenye jukwaa hili".to_string(),
                ))
            }
        }),
    );
    m.insert(
        "rejesha_ishara".to_string(),
        Box::new(|args: &[Value]| {
            #[cfg(unix)]
            {
                let sig_id =
                    value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0) as i32;
                signal::clear_handler(sig_id);
                Ok(Value::sawa(Value::Tupu))
            }
            #[cfg(not(unix))]
            {
                Ok(Value::kosa(
                    "rejesha_ishara: sifa haipo kwenye jukwaa hili".to_string(),
                ))
            }
        }),
    );
}

/// The watchdog behind `mlinzi_anza`/`mlinzi_lisha`: one thread per process, checking the
/// deadline the program keeps pushing forward.
mod mlinzi {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    /// (interval, deadline) once armed.
    static STATE: Mutex<Option<(Duration, Instant)>> = Mutex::new(None);
    static THREAD: std::sync::Once = std::sync::Once::new();

    pub(super) fn anza(interval: Duration) {
        *crate::sync::lock(&STATE) = Some((interval, Instant::now() + interval));
        #[cfg(not(target_arch = "wasm32"))]
        THREAD.call_once(|| {
            std::thread::spawn(|| loop {
                let deadline = crate::sync::lock(&STATE).map(|(_, d)| d);
                let Some(deadline) = deadline else { return };
                let now = Instant::now();
                if now >= deadline {
                    crate::hali_salama::enter("mlinzi: muda umekwisha bila kulishwa");
                    eprintln!("mlinzi: muda umekwisha bila kulishwa");
                    crate::platform::flush_stdout();
                    std::process::exit(5);
                }
                std::thread::sleep((deadline - now).min(Duration::from_millis(10)));
            });
        });
    }

    pub(super) fn lisha() {
        if let Some((interval, deadline)) = crate::sync::lock(&STATE).as_mut() {
            *deadline = Instant::now() + *interval;
        }
    }
}
