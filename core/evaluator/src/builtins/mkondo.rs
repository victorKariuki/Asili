//! Mkondo (network stream/socket): mkondo_unganisha (connect a TCP client stream). Returns
//! Tokeo<Mkondo, Neno>. The handle closes automatically on drop (scope exit, explicit tupa, or
//! .funga()) via MkondoHandle's own Drop impl — see docs/design/faili-mkondo-design.md.

use std::cell::RefCell;
use std::collections::HashMap;
use std::net::TcpStream;
use std::rc::Rc;

use crate::value::{self, MkondoHandle, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("mkondo_unganisha".to_string(), Box::new(|args: &[Value]| {
        let addr = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            match TcpStream::connect(&addr) {
                Ok(s) => Ok(Value::Tokeo(Ok(Box::new(Value::Mkondo(Rc::new(RefCell::new(MkondoHandle(Some(s))))))))),
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "mkondo_unganisha: haipatikani kwenye kivinjari".into(),
        )))))
    }));
}
