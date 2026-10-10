//! Msingi (prelude): orodha, orodha_rudia, kamusi, kamusi_tupu, jozi, tokeo, kosa, chaguo.

use std::collections::HashMap;

use super::BuiltinFn;
use crate::value::Value;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "baiti".to_string(),
        Box::new(|args: &[Value]| {
            let list = args.first().cloned().unwrap_or(Value::Hamna);
            Ok(match crate::eval::methods::bytes_from_list(&list) {
                Ok(b) => Value::sawa(Value::Baiti(b.into())),
                Err(e) => Value::kosa(e),
            })
        }),
    );
    m.insert(
        "baiti_ya_nambari".to_string(),
        Box::new(|args: &[Value]| {
            let n = crate::value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(-1.0);
            let width = crate::value::as_f64(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or(0.0);
            let little = crate::value::as_string(args.get(2).unwrap_or(&Value::Hamna))
                .is_some_and(|m| m == "le");
            let width = width as usize;
            let max = if width == 8 {
                u64::MAX as f64
            } else {
                (1u128 << (8 * width)) as f64 - 1.0
            };
            Ok(if !matches!(width, 1 | 2 | 4 | 8) {
                Value::kosa("baiti_ya_nambari: upana ni 1, 2, 4 au 8")
            } else if n.fract() != 0.0 || n < 0.0 || n > max {
                Value::kosa(format!("baiti_ya_nambari: {n} haitoshei baiti {width}"))
            } else {
                let full = if little {
                    (n as u64).to_le_bytes()
                } else {
                    (n as u64).to_be_bytes()
                };
                let bytes = if little {
                    &full[..width]
                } else {
                    &full[8 - width..]
                };
                Value::sawa(Value::Baiti(bytes.into()))
            })
        }),
    );
    m.insert(
        "orodha".to_string(),
        Box::new(|args: &[Value]| Ok(Value::list(args.to_vec()))),
    );
    m.insert(
        "orodha_rudia".to_string(),
        Box::new(|args: &[Value]| {
            let value = args.first().cloned().unwrap_or(Value::Hamna);
            let count = match args.get(1) {
                Some(Value::Namba(n)) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => {
                    *n as usize
                }
                _ => {
                    return Err(crate::value::EvalError::TypeErr(
                        "orodha_rudia inahitaji idadi ya Namba kamili isiyo hasi".to_string(),
                    ));
                }
            };
            Ok(Value::list(std::iter::repeat_n(value, count).collect()))
        }),
    );
    m.insert(
        "kamusi".to_string(),
        Box::new(|_args: &[Value]| Ok(Value::Kamusi(Default::default()))),
    );
    m.insert(
        "kamusi_tupu".to_string(),
        Box::new(|_args: &[Value]| Ok(Value::Kamusi(Default::default()))),
    );
    m.insert(
        "jozi".to_string(),
        Box::new(|args: &[Value]| {
            let a = args.first().cloned().unwrap_or(Value::Hamna);
            let b = args.get(1).cloned().unwrap_or(Value::Hamna);
            Ok(Value::Jozi(Box::new(a), Box::new(b)))
        }),
    );
    m.insert(
        "ok".to_string(),
        Box::new(|args: &[Value]| {
            let val = args.first().cloned().unwrap_or(Value::Hamna);
            Ok(Value::sawa(val))
        }),
    );
    m.insert(
        "tokeo".to_string(),
        Box::new(|args: &[Value]| {
            let val = args.first().cloned().unwrap_or(Value::Hamna);
            Ok(Value::sawa(val))
        }),
    );
    m.insert(
        "kosa".to_string(),
        Box::new(|args: &[Value]| {
            // Accept any value as error (not just strings)
            let err = args
                .first()
                .cloned()
                .unwrap_or(Value::neno("error".to_string()));
            Ok(Value::Tokeo(Err(Box::new(err))))
        }),
    );
    m.insert(
        "chaguo".to_string(),
        Box::new(|args: &[Value]| {
            let val = args.first().cloned().unwrap_or(Value::Hamna);
            Ok(Value::Chaguo(Some(Box::new(val))))
        }),
    );
}
