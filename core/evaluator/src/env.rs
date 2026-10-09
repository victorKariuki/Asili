//! The language's predefined names.

use crate::value::Value;

/// The language's predefined names (`Ukomo`, `Siyo_Namba`, `PI`, `KWELI`, `TOLEO`, …): what the
/// bytecode compiler resolves a name to when it is neither local nor a module constant.
pub(crate) fn global_constants() -> Vec<(&'static str, Value)> {
    vec![
        ("KWELI", Value::Ukweli(true)),
        ("SIYO_KWELI", Value::Ukweli(false)),
        ("TUPU", Value::Tupu),
        ("Ukomo", Value::Namba(f64::INFINITY)),
        ("Siyo_Namba", Value::Namba(f64::NAN)),
        (
            "TOLEO",
            Value::Neno(option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0").into()),
        ),
        ("JINA_OS", Value::Neno(std::env::consts::OS.into())),
        ("SEKUNDE_KWA_SIKU", Value::Namba(86400.0)),
        ("MWANZO_WA_ZAMANI", Value::Namba(0.0)),
        (
            "NJIA_SEPARATOR",
            Value::neno(std::path::MAIN_SEPARATOR.to_string()),
        ),
        ("PI", Value::Namba(std::f64::consts::PI)),
        ("E", Value::Namba(std::f64::consts::E)),
        ("PHI", Value::Namba((1.0_f64 + 5.0_f64.sqrt()) / 2.0)),
        ("TAU", Value::Namba(2.0 * std::f64::consts::PI)),
        ("LN10", Value::Namba(10.0_f64.ln())),
        ("LN2", Value::Namba(2.0_f64.ln())),
        ("LOG10E", Value::Namba(std::f64::consts::E.log10())),
        ("LOG2E", Value::Namba(std::f64::consts::E.log2())),
        ("KIPEUO1_2", Value::Namba(1.0 / 2.0_f64.sqrt())),
        ("KIPEUO2", Value::Namba(2.0_f64.sqrt())),
        ("KIPEUO3", Value::Namba(3.0_f64.sqrt())),
        ("KIPEUO5", Value::Namba(5.0_f64.sqrt())),
        ("EPSILON", Value::Namba(f64::EPSILON)),
        ("INF", Value::Namba(f64::INFINITY)),
        ("NAN", Value::Namba(f64::NAN)),
    ]
}
