//! Msingi (prelude): orodha, kamusi, kamusi_tupu, jozi, tokeo, kosa, chaguo.

use std::collections::HashMap;

use crate::value::Value;
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("orodha".to_string(), Box::new(|args: &[Value]| {
        Ok(Value::Orodha(args.to_vec()))
    }));
    m.insert("kamusi".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Kamusi(HashMap::new()))
    }));
    m.insert("kamusi_tupu".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Kamusi(HashMap::new()))
    }));
    m.insert("jozi".to_string(), Box::new(|args: &[Value]| {
        let a = args.get(0).cloned().unwrap_or(Value::Hamna);
        let b = args.get(1).cloned().unwrap_or(Value::Hamna);
        Ok(Value::Jozi(Box::new(a), Box::new(b)))
    }));
    m.insert("ok".to_string(), Box::new(|args: &[Value]| {
        let val = args.first().cloned().unwrap_or(Value::Hamna);
        Ok(Value::Tokeo(Ok(Box::new(val))))
    }));
    m.insert("tokeo".to_string(), Box::new(|args: &[Value]| {
        let val = args.first().cloned().unwrap_or(Value::Hamna);
        Ok(Value::Tokeo(Ok(Box::new(val))))
    }));
    m.insert("kosa".to_string(), Box::new(|args: &[Value]| {
        // Accept any value as error (not just strings)
        let err = args.first().cloned().unwrap_or(Value::Neno("error".to_string()));
        Ok(Value::Tokeo(Err(Box::new(err))))
    }));
    m.insert("chaguo".to_string(), Box::new(|args: &[Value]| {
        let val = args.first().cloned().unwrap_or(Value::Hamna);
        Ok(Value::Chaguo(Some(Box::new(val))))
    }));
}
