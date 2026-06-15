//! Matumizi (I/O): chapisha, onyo, makosa, paparika, omba.

use std::collections::HashMap;
use std::env;
use std::io;

use crate::value::{self, Value, EvalError};
use super::BuiltinFn;

fn format_message_for_display(msg: &str) -> &str {
    if msg.len() >= 2 && msg.starts_with('"') && msg.ends_with('"') {
        &msg[1..msg.len() - 1]
    } else {
        msg
    }
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("chapisha".to_string(), Box::new(|args: &[Value]| {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(Value::Neno(ref msg)) = args.first() {
            let display = format_message_for_display(msg);
            println!("{display}");
        }
        #[cfg(target_arch = "wasm32")]
        let _ = args;
        Ok(Value::Tupu)
    }));
    m.insert("onyo".to_string(), Box::new(|args: &[Value]| {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let msg = args
                .first()
                .and_then(value::as_string)
                .unwrap_or_default();
            let display = format_message_for_display(&msg);
            let line = if env::var("ASILI_TELEMETRY").is_ok() {
                format!("[WARN][SUBSTRATE] {display}")
            } else {
                display.to_string()
            };
            eprintln!("{line}");
        }
        #[cfg(target_arch = "wasm32")]
        let _ = args;
        Ok(Value::Tupu)
    }));
    m.insert("makosa".to_string(), Box::new(|args: &[Value]| {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let msg = args
                .first()
                .and_then(value::as_string)
                .unwrap_or_default();
            let display = format_message_for_display(&msg);
            let line = if env::var("ASILI_TELEMETRY").is_ok() {
                format!("[ERR][CRITICAL] {display}")
            } else {
                format!("KOSA: {display}")
            };
            eprintln!("{line}");
        }
        #[cfg(target_arch = "wasm32")]
        let _ = args;
        Ok(Value::Tupu)
    }));
    m.insert("paparika".to_string(), Box::new(|args: &[Value]| {
        let msg = args
            .first()
            .and_then(value::as_string)
            .unwrap_or_else(|| "paparika".to_string());
        Err(EvalError::Panic(msg))
    }));
    m.insert("omba".to_string(), Box::new(|_args: &[Value]| {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let stdin = io::stdin();
            let mut line = String::new();
            match stdin.read_line(&mut line) {
                Ok(_) => {
                    if line.ends_with('\n') {
                        line.pop();
                    }
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    Ok(Value::Neno(line))
                }
                Err(e) => Err(EvalError::Panic(format!("omba: read error: {}", e))),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err(EvalError::Panic("omba: stdin not available in WASM".to_string()))
        }
    }));
}
