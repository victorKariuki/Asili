//! Msingi (prelude): orodha, orodha_rudia, kamusi, kamusi_tupu, jozi, tokeo, kosa, chaguo.

use std::collections::HashMap;

use super::BuiltinFn;
use crate::value::Value;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
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
        Box::new(|_args: &[Value]| Ok(Value::Kamusi(HashMap::new()))),
    );
    m.insert(
        "kamusi_tupu".to_string(),
        Box::new(|_args: &[Value]| Ok(Value::Kamusi(HashMap::new()))),
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
