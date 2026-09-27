//! Hisabati (math) builtins.

use std::collections::HashMap;

use rand::RngExt;

use super::BuiltinFn;
use crate::value::{self, Value};

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "namba_kuu_kutoka".to_string(),
        Box::new(|args: &[Value]| {
            use crate::value::BigInt;
            use std::str::FromStr;
            let s = super::arg_str(args, 0);
            match BigInt::from_str(s.trim()) {
                Ok(n) => Ok(Value::sawa(Value::NambaKuu(n))),
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
                Ok(n) => Ok(Value::sawa(Value::NambaSahihi(n))),
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
        "duara".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "duara")?;
            Ok(Value::Namba(n.round()))
        }),
    );
    m.insert(
        "absolute".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "absolute")?;
            Ok(Value::Namba(n.abs()))
        }),
    );
    m.insert(
        "kipeo".to_string(),
        Box::new(|args: &[Value]| {
            let (n, p) = value::args_f64_2(args, "kipeo")?;
            let r = n.powf(p);
            if r.is_finite() {
                Ok(Value::sawa(Value::Namba(r)))
            } else {
                Ok(Value::kosa("kipeo: matokeo si mwisho"))
            }
        }),
    );
    m.insert(
        "mizizi".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "mizizi")?;
            if n < 0.0 {
                Ok(Value::kosa("mizizi: namba hasi"))
            } else {
                Ok(Value::sawa(Value::Namba(n.sqrt())))
            }
        }),
    );
    m.insert(
        "abs".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "abs")?;
            Ok(Value::Namba(n.abs()))
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
        "upeo".to_string(),
        Box::new(|args: &[Value]| {
            let (n, p) = value::args_f64_2(args, "upeo")?;
            let r = n.powf(p);
            if r.is_finite() {
                Ok(Value::sawa(Value::Namba(r)))
            } else {
                Ok(Value::kosa("upeo: matokeo si mwisho"))
            }
        }),
    );
    m.insert(
        "kipeuo2".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "kipeuo2")?;
            if n < 0.0 {
                Ok(Value::kosa("kipeuo2: namba hasi"))
            } else {
                Ok(Value::sawa(Value::Namba(n.sqrt())))
            }
        }),
    );
    m.insert(
        "kipeuo3".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "kipeuo3")?;
            Ok(Value::Namba(n.cbrt()))
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
        "upeo_wa_e".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "upeo_wa_e")?;
            Ok(Value::Namba(x.exp()))
        }),
    );
    m.insert(
        "expm1".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "expm1")?;
            Ok(Value::Namba(x.exp_m1()))
        }),
    );
    m.insert(
        "sakafu".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "sakafu")?;
            Ok(Value::Namba(n.floor()))
        }),
    );
    m.insert(
        "dari".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "dari")?;
            Ok(Value::Namba(n.ceil()))
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
        "punguza".to_string(),
        Box::new(|args: &[Value]| {
            let n = value::arg_f64(args, 0, "punguza")?;
            Ok(Value::Namba(n.trunc()))
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
        "logi".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "logi")?;
            if x <= 0.0 {
                return Ok(Value::kosa("logi: x lazima iwe chanya"));
            }
            Ok(Value::sawa(Value::Namba(x.ln())))
        }),
    );
    m.insert(
        "logi10".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "logi10")?;
            if x <= 0.0 {
                return Ok(Value::kosa("logi10: x lazima iwe chanya"));
            }
            Ok(Value::sawa(Value::Namba(x.log10())))
        }),
    );
    m.insert(
        "logi2".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "logi2")?;
            if x <= 0.0 {
                return Ok(Value::kosa("logi2: x lazima iwe chanya"));
            }
            Ok(Value::sawa(Value::Namba(x.log2())))
        }),
    );
    m.insert(
        "logi1p".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "logi1p")?;
            if x <= -1.0 {
                return Ok(Value::kosa("logi1p: x lazima iwe kubwa kuliko -1"));
            }
            Ok(Value::sawa(Value::Namba(x.ln_1p())))
        }),
    );
    m.insert(
        "asini".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "asini")?;
            if !(-1.0..=1.0).contains(&x) {
                return Ok(Value::kosa("asini: kikoa ni -1 hadi 1"));
            }
            Ok(Value::sawa(Value::Namba(x.asin())))
        }),
    );
    m.insert(
        "akosini".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "akosini")?;
            if !(-1.0..=1.0).contains(&x) {
                return Ok(Value::kosa("akosini: kikoa ni -1 hadi 1"));
            }
            Ok(Value::sawa(Value::Namba(x.acos())))
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
        "akosini_h".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "akosini_h")?;
            if x < 1.0 {
                return Ok(Value::kosa("akosini_h: x lazima iwe >= 1"));
            }
            Ok(Value::sawa(Value::Namba(x.acosh())))
        }),
    );
    m.insert(
        "atanjenti_h".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "atanjenti_h")?;
            if x <= -1.0 || x >= 1.0 {
                return Ok(Value::kosa("atanjenti_h: kikoa ni -1 < x < 1"));
            }
            Ok(Value::sawa(Value::Namba(x.atanh())))
        }),
    );
    m.insert(
        "sini".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "sini")?;
            Ok(Value::Namba(x.sin()))
        }),
    );
    m.insert(
        "kosini".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "kosini")?;
            Ok(Value::Namba(x.cos()))
        }),
    );
    m.insert(
        "tanjenti".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "tanjenti")?;
            Ok(Value::Namba(x.tan()))
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
        "sini_h".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "sini_h")?;
            Ok(Value::Namba(x.sinh()))
        }),
    );
    m.insert(
        "kosini_h".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "kosini_h")?;
            Ok(Value::Namba(x.cosh()))
        }),
    );
    m.insert(
        "tanjenti_h".to_string(),
        Box::new(|args: &[Value]| {
            let x = value::arg_f64(args, 0, "tanjenti_h")?;
            Ok(Value::Namba(x.tanh()))
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
            let r: f64 = rand::rng().random();
            Ok(Value::Namba(r))
        }),
    );
    m.insert(
        "nasibu_chini".to_string(),
        Box::new(|args: &[Value]| {
            let (min, max) = value::args_f64_2(args, "nasibu_chini")?;
            let r: f64 = rand::rng().random();
            Ok(Value::Namba(min + r * (max - min)))
        }),
    );
}
