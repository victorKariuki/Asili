//! `Kumbukumbu<T>` (heap box): kumbukumbu_unda. No OS resource — plain owning indirection, so no
//! special drop handling is needed beyond Box's own. See docs/design/faili-mkondo-design.md.

use std::collections::HashMap;

use crate::value::Value;
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("kumbukumbu_unda".to_string(), Box::new(|args: &[Value]| {
        let inner = args.first().cloned().unwrap_or(Value::Hamna);
        Ok(Value::Kumbukumbu(Box::new(inner)))
    }));
}
