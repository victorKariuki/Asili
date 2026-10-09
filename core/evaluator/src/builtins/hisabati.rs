//! Hisabati (math) builtins.

use std::collections::HashMap;

use rand::RngExt;

use super::BuiltinFn;
use crate::value::{self, Value};

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    // Total unary functions: Namba -> Namba.
    const UNARY: &[(&str, fn(f64) -> f64)] = &[
        ("duara", f64::round),
        ("absolute", f64::abs),
        ("abs", f64::abs),
        ("kipeuo3", f64::cbrt),
        ("upeo_wa_e", f64::exp),
        ("expm1", f64::exp_m1),
        ("sakafu", f64::floor),
        ("dari", f64::ceil),
        ("punguza", f64::trunc),
        ("sini", f64::sin),
        ("kosini", f64::cos),
        ("tanjenti", f64::tan),
        ("sini_h", f64::sinh),
        ("kosini_h", f64::cosh),
        ("tanjenti_h", f64::tanh),
    ];
    for &(name, f) in UNARY {
        m.insert(
            name.to_string(),
            Box::new(move |args: &[Value]| Ok(Value::Namba(f(value::arg_f64(args, 0, name)?)))),
        );
    }
    // Partial unary functions: Namba -> Tokeo<Namba, Neno>, KOSA outside the domain.
    type Checked = (&'static str, fn(f64) -> bool, &'static str, fn(f64) -> f64);
    const CHECKED: &[Checked] = &[
        ("logi", |x| x <= 0.0, "x lazima iwe chanya", f64::ln),
        ("logi10", |x| x <= 0.0, "x lazima iwe chanya", f64::log10),
        ("logi2", |x| x <= 0.0, "x lazima iwe chanya", f64::log2),
        (
            "logi1p",
            |x| x <= -1.0,
            "x lazima iwe kubwa kuliko -1",
            f64::ln_1p,
        ),
        (
            "asini",
            |x| !(-1.0..=1.0).contains(&x),
            "kikoa ni -1 hadi 1",
            f64::asin,
        ),
        (
            "akosini",
            |x| !(-1.0..=1.0).contains(&x),
            "kikoa ni -1 hadi 1",
            f64::acos,
        ),
        ("akosini_h", |x| x < 1.0, "x lazima iwe >= 1", f64::acosh),
        (
            "atanjenti_h",
            |x| x <= -1.0 || x >= 1.0,
            "kikoa ni -1 < x < 1",
            f64::atanh,
        ),
        ("mizizi", |x| x < 0.0, "namba hasi", f64::sqrt),
        ("kipeuo2", |x| x < 0.0, "namba hasi", f64::sqrt),
    ];
    for &(name, outside_domain, msg, f) in CHECKED {
        m.insert(
            name.to_string(),
            Box::new(move |args: &[Value]| {
                let x = value::arg_f64(args, 0, name)?;
                if outside_domain(x) {
                    return Ok(Value::kosa(format!("{name}: {msg}")));
                }
                Ok(Value::sawa(Value::Namba(f(x))))
            }),
        );
    }
    register_pow(m, "kipeo");
    register_pow(m, "upeo");
    m.insert(
        "namba_kuu_kutoka".to_string(),
        Box::new(|args: &[Value]| {
            use crate::value::BigInt;
            use std::str::FromStr;
            let s = super::arg_str(args, 0);
            match BigInt::from_str(s.trim()) {
                Ok(n) => Ok(Value::sawa(Value::NambaKuu(std::rc::Rc::new(n)))),
                Err(_) => Ok(Value::kosa(format!(
                    "namba_kuu_kutoka: \"{s}\" si namba kamili sahihi"
                ))),
            }
        }),
    );
    m.insert(
        "namba_sahihi_kutoka".to_string(),
        Box::new(|args: &[Value]| {
            use crate::value::BigDecimal;
            use std::str::FromStr;
            let s = super::arg_str(args, 0);
            match BigDecimal::from_str(s.trim()) {
                Ok(n) => Ok(Value::sawa(Value::NambaSahihi(std::rc::Rc::new(n)))),
                Err(_) => Ok(Value::kosa(format!(
                    "namba_sahihi_kutoka: \"{s}\" si namba ya desimali sahihi"
                ))),
            }
        }),
    );
    m.insert(
        "jumla".to_string(),
        Box::new(|args: &[Value]| {
            let (a, b) = value::args_f64_2(args, "jumla")?;
            Ok(Value::Namba(a + b))
        }),
    );
    m.insert(
        "tofauti".to_string(),
        Box::new(|args: &[Value]| {
            let (a, b) = value::args_f64_2(args, "tofauti")?;
            Ok(Value::Namba(a - b))
        }),
    );
    m.insert(
        "zao".to_string(),
        Box::new(|args: &[Value]| {
            let (a, b) = value::args_f64_2(args, "zao")?;
            Ok(Value::Namba(a * b))
        }),
    );
    m.insert(
        "gawio".to_string(),
        Box::new(|args: &[Value]| {
            let (a, b) = value::args_f64_2(args, "gawio")?;
            if b == 0.0 {
                return Ok(Value::kosa("gawio kwa sifuri"));
            }
            Ok(Value::sawa(Value::Namba(a / b)))
        }),
    );
    m.insert(
        "ishara".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "ishara")?;
            let s = if n > 0.0 {
                1.0
            } else if n < 0.0 {
                -1.0
            } else {
                0.0
            };
            Ok(Value::Namba(s))
        }),
    );
    m.insert(
        "haipot".to_string(),
        Box::new(|args: &[Value]| {
            let (x, y) = value::args_f64_2(args, "haipot")?;
            Ok(Value::Namba((x * x + y * y).sqrt()))
        }),
    );
    m.insert(
        "duara_maeneo".to_string(),
        Box::new(|args: &[Value]| {
            let (x, m) = value::args_f64_2(args, "duara_maeneo")?;
            let factor = 10.0_f64.powi(m as i32);
            Ok(Value::Namba((x * factor).round() / factor))
        }),
    );
    m.insert(
        "kubwa".to_string(),
        Box::new(|args: &[Value]| {
            let (a, b) = value::args_f64_2(args, "kubwa")?;
            Ok(Value::Namba(a.max(b)))
        }),
    );
    m.insert(
        "ndogo".to_string(),
        Box::new(|args: &[Value]| {
            let (a, b) = value::args_f64_2(args, "ndogo")?;
            Ok(Value::Namba(a.min(b)))
        }),
    );
    m.insert(
        "kikwazo".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "kikwazo")?;
            let ch = value::arg_f64(args, 1, "kikwazo")?;
            let j = value::arg_f64(args, 2, "kikwazo")?;
            Ok(Value::Namba(x.max(ch).min(j)))
        }),
    );
    m.insert(
        "kwenda_radiani".to_string(),
        Box::new(|args: &[Value]| {
            let d = value::arg_f64(args, 0, "kwenda_radiani")?;
            Ok(Value::Namba(d * std::f64::consts::PI / 180.0))
        }),
    );
    m.insert(
        "kwenda_nyuzi".to_string(),
        Box::new(|args: &[Value]| {
            let r = value::arg_f64(args, 0, "kwenda_nyuzi")?;
            Ok(Value::Namba(r * 180.0 / std::f64::consts::PI))
        }),
    );
    m.insert(
        "mzizi".to_string(),
        Box::new(|args: &[Value]| {
            let (x, n) = value::args_f64_2(args, "mzizi")?;
            if n <= 0.0 {
                return Ok(Value::kosa("mzizi: n lazima ni kubwa kuliko 0"));
            }
            if x < 0.0 && (n as i32 as f64 - n).abs() < f64::EPSILON && (n as i32) % 2 == 0 {
                return Ok(Value::kosa(
                    "mzizi: mzizi wa hata kwa namba hasi haujulikani",
                ));
            }
            let r = x.powf(1.0 / n);
            Ok(Value::sawa(Value::Namba(r)))
        }),
    );
    m.insert(
        "faktoriali".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "faktoriali")?;
            if n < 0.0 || n.fract().abs() > f64::EPSILON {
                return Ok(Value::kosa("faktoriali: n lazima iwe namba nzima si hasi"));
            }
            let mut r = 1.0_f64;
            for i in 1..=n as u64 {
                r *= i as f64;
            }
            Ok(Value::sawa(Value::Namba(r)))
        }),
    );
    m.insert(
        "baki".to_string(),
        Box::new(|args: &[Value]| {
            let (x, y) = value::args_f64_2(args, "baki")?;
            if y == 0.0 {
                return Ok(Value::kosa("baki: mgawanyiko kwa sifuri"));
            }
            Ok(Value::sawa(Value::Namba(x % y)))
        }),
    );
    m.insert(
        "atanjenti".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "atanjenti")?;
            Ok(Value::sawa(Value::Namba(x.atan())))
        }),
    );
    m.insert(
        "asini_h".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "asini_h")?;
            Ok(Value::sawa(Value::Namba(x.asinh())))
        }),
    );
    m.insert(
        "atanjenti2".to_string(),
        Box::new(|args: &[Value]| {
            let (y, x) = value::args_f64_2(args, "atanjenti2")?;
            Ok(Value::Namba(y.atan2(x)))
        }),
    );
    m.insert(
        "ni_namba".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "ni_namba")?;
            Ok(Value::Ukweli(x.is_finite()))
        }),
    );
    m.insert(
        "si_namba".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "si_namba")?;
            Ok(Value::Ukweli(x.is_nan()))
        }),
    );
    m.insert(
        "ni_ukomo".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "ni_ukomo")?;
            Ok(Value::Ukweli(x.is_infinite()))
        }),
    );
    m.insert(
        "nasibu".to_string(),
        Box::new(|args: &[Value]| {
            if !args.is_empty() {
                return Err(crate::value::EvalError::TypeErr(format!(
                    "nasibu: haihitaji hoja, umeweka {}",
                    args.len()
                )));
            }
            Ok(Value::Namba(random_f64()))
        }),
    );
    m.insert(
        "nasibu_chini".to_string(),
        Box::new(|args: &[Value]| {
            let (min, max) = value::args_f64_2(args, "nasibu_chini")?;
            Ok(Value::Namba(min + random_f64() * (max - min)))
        }),
    );
    m.insert(
        "nasibu_kamili".to_string(),
        Box::new(|args: &[Value]| {
            let (a, b) = value::args_f64_2(args, "nasibu_kamili")?;
            let (lo, hi) = (a.min(b).ceil(), a.max(b).floor());
            if !(lo.is_finite() && hi.is_finite()) || lo > hi || hi - lo >= 9.0e15 {
                return Err(crate::value::EvalError::TypeErr(
                    "nasibu_kamili inahitaji mipaka ya Namba kamili zenye nafasi kati yao".into(),
                ));
            }
            Ok(Value::Namba(lo + random_below((hi - lo) as u64 + 1) as f64))
        }),
    );
    m.insert(
        "nasibu_mbegu".to_string(),
        Box::new(|args: &[Value]| {
            let seed = value::as_f64(args.first().unwrap_or(&Value::Hamna)).ok_or_else(|| {
                crate::value::EvalError::TypeErr("nasibu_mbegu inahitaji Namba".into())
            })?;
            seed_rng(seed);
            Ok(Value::Tupu)
        }),
    );
    m.insert(
        "changanya".to_string(),
        Box::new(|args: &[Value]| {
            let mut items = match args.first() {
                Some(Value::Orodha(l)) => l.to_vec(),
                _ => {
                    return Err(crate::value::EvalError::TypeErr(
                        "changanya inahitaji Orodha".into(),
                    ))
                }
            };
            // Fisher–Yates.
            for i in (1..items.len()).rev() {
                let j = random_below(i as u64 + 1) as usize;
                items.swap(i, j);
            }
            Ok(Value::list(items))
        }),
    );
    m.insert(
        "chagua_nasibu".to_string(),
        Box::new(|args: &[Value]| match args.first() {
            Some(Value::Orodha(l)) if l.is_empty() => Ok(Value::Chaguo(None)),
            Some(Value::Orodha(l)) => {
                let i = random_below(l.len() as u64) as usize;
                Ok(Value::Chaguo(l.get(i).cloned().map(Box::new)))
            }
            _ => Err(crate::value::EvalError::TypeErr(
                "chagua_nasibu inahitaji Orodha".into(),
            )),
        }),
    );
}

thread_local! {
    /// This thread's random numbers after `nasibu_mbegu` (a reproducible sequence), or `None`
    /// for the system's (seeded from the operating system).
    static SEEDED: std::cell::RefCell<Option<rand::rngs::StdRng>> =
        const { std::cell::RefCell::new(None) };
}

fn seed_rng(seed: f64) {
    use rand::SeedableRng;
    SEEDED.with(|s| *s.borrow_mut() = Some(rand::rngs::StdRng::seed_from_u64(seed.to_bits())));
}

/// A number in `[0, 1)` from this thread's random numbers.
pub(crate) fn random_f64() -> f64 {
    SEEDED.with(|s| match &mut *s.borrow_mut() {
        Some(r) => r.random(),
        None => rand::rng().random(),
    })
}

/// A whole number in `[0, n)` (`n` > 0), uniformly.
pub(crate) fn random_below(n: u64) -> u64 {
    SEEDED.with(|s| match &mut *s.borrow_mut() {
        Some(r) => r.random_range(0..n),
        None => rand::rng().random_range(0..n),
    })
}

/// `n` random bytes from this thread's random numbers.
pub(crate) fn random_bytes<const N: usize>() -> [u8; N] {
    SEEDED.with(|s| match &mut *s.borrow_mut() {
        Some(r) => r.random(),
        None => rand::rng().random(),
    })
}

/// `n ** p` as `Tokeo`, KOSA when the result is not finite.
fn register_pow(m: &mut HashMap<String, BuiltinFn>, name: &'static str) {
    m.insert(
        name.to_string(),
        Box::new(move |args: &[Value]| {
            let (n, p) = value::args_f64_2(args, name)?;
            let r = n.powf(p);
            if r.is_finite() {
                Ok(Value::sawa(Value::Namba(r)))
            } else {
                Ok(Value::kosa(format!("{name}: matokeo si mwisho")))
            }
        }),
    );
}
