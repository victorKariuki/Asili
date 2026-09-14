//! Sambamba (concurrency): tenda/subiri_tenda (thread spawn/join), njia (channel), fungo
//! (mutex). 1:1 OS-thread model (see docs/design/concurrency-design.md for why, and why not
//! the spec's "green threads" framing taken literally as M:N).
//!
//! `tenda` looks up a module-level `kazi` by name and spawns a real OS thread (`std::thread`)
//! running it with an independently-owned clone of the current `Module` — not a borrow of the
//! caller's `Runtime`, which isn't `Send` (it borrows `Env`/`Module` by reference).
//!
//! **Neither `tenda`'s arguments nor a spawned function's return value cross the thread
//! boundary as a plain `Value`.** `Value` as a whole is not unconditionally `Send` (some
//! variants — `KashaGC`/`Faili`/`Mkondo`, and transitively `Kumbukumbu` since it can box any of
//! them — hold `Rc`, deliberately not `Send`), so `std::thread::spawn`'s `F: Send + 'static`
//! bound rejects a closure that merely *captures* a `Vec<Value>`, even one a runtime check has
//! already confirmed contains no non-Send variant — Rust's check is on the static type, not a
//! specific value. Rather than reach for `unsafe impl Send` on a wrapper (sound only as long as
//! every future `Rc`-based `Value` variant remembers to be excluded — a real, recurring risk in
//! a codebase that added three such variants in one session), arguments and the return value
//! both convert through `Value::try_into_send()` / `SendValue::into_value()`
//! (`core/evaluator/src/value/mod.rs`) — a structural, compiler-checked `Send`-safe mirror of
//! `Value`, not a runtime-only assertion. `Kasha_GC`/`Faili`/`Mkondo`/`Kumbukumbu` values (or
//! anything transitively containing one) are rejected with a clear error instead of silently
//! producing undefined behavior or a panic deep inside another thread; `njia`/`fungo` (built on
//! `Arc<Mutex<_>>`, genuinely `Send` on their own) are how a spawned function's real results are
//! meant to flow back — `subiri_tenda` only reports whether the thread finished cleanly.
//!
//! `tenda` itself is NOT registered as an ordinary BuiltinFn (`Fn(&[Value]) -> ...` has no
//! access to the current Module) — it's special-cased in eval/expr.rs's Expr::Call handling,
//! which has `rt.module` available, and calls `tenda()` below directly.

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

use asili_parser::Module;

use crate::value::{self, EvalError, Value};
use super::BuiltinFn;

static NEXT_HANDLE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn handles() -> &'static Mutex<HashMap<u64, JoinHandle<Result<(), String>>>> {
    static HANDLES: OnceLock<Mutex<HashMap<u64, JoinHandle<Result<(), String>>>>> = OnceLock::new();
    HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `tenda(kazi_jina, hoja...) -> Tokeo<Namba, Neno>` — spawns `kazi_jina` (a module-level `kazi`
/// found in `module`) on a new OS thread with a `.clone()` of `module` (Module is plain owned
/// data — Send — so this is a cheap, correct way to give the thread its own copy rather than
/// trying to share the caller's borrowed Runtime). Returns a handle id on success. The spawned
/// function's own return value is discarded (see module doc comment) — communicate results back
/// via `njia`.
pub(crate) fn tenda(module: &Module, args: &[Value]) -> Result<Value, EvalError> {
    let kazi_name = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
    let raw_args = args.get(1..).unwrap_or(&[]);
    let call_args: Vec<value::SendValue> = match raw_args.iter().map(Value::try_into_send).collect() {
        Some(v) => v,
        None => {
            return Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "tenda: hoja ina thamani isiyoweza kuvuka nyuzi (Kasha_GC/Faili/Mkondo) — tumia njia/fungo badala yake".to_string(),
            )))));
        }
    };
    if !module.functions.iter().any(|f| f.name == kazi_name) {
        return Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!(
            "tenda: kazi haijulikani: {kazi_name}"
        ))))));
    }
    let module_owned = module.clone();
    let handle = std::thread::spawn(move || {
        let call_args: Vec<Value> = call_args.into_iter().map(value::SendValue::into_value).collect();
        crate::run_function(&module_owned, &kazi_name, call_args)
            .map(|_| ())
            .map_err(|e| e.to_string())
    });
    let id = NEXT_HANDLE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    handles().lock().unwrap().insert(id, handle);
    Ok(Value::Tokeo(Ok(Box::new(Value::Namba(id as f64)))))
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("subiri_tenda".to_string(), Box::new(|args: &[Value]| {
        let id = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0) as u64;
        let handle = handles().lock().unwrap().remove(&id);
        match handle {
            None => Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!(
                "subiri_tenda: uzi haujulikani au tayari umesubiriwa: {id}"
            )))))),
            Some(h) => match h.join() {
                Ok(Ok(())) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                Ok(Err(msg)) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(msg))))),
                Err(_) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "subiri_tenda: uzi ulianguka (panic)".to_string(),
                ))))),
            },
        }
    }));
    m.insert("njia".to_string(), Box::new(|_args: &[Value]| {
        let (tx, rx) = mpsc::channel::<value::SendValue>();
        Ok(Value::Jozi(
            Box::new(Value::NjiaTx(Arc::new(Mutex::new(tx)))),
            Box::new(Value::NjiaRx(Arc::new(Mutex::new(rx)))),
        ))
    }));
    m.insert("njia_na_kikomo".to_string(), Box::new(|args: &[Value]| {
        let kikomo = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0).max(0.0) as usize;
        let (tx, rx) = mpsc::sync_channel::<value::SendValue>(kikomo);
        Ok(Value::Jozi(
            Box::new(Value::NjiaTxBounded(Arc::new(Mutex::new(tx)))),
            Box::new(Value::NjiaRxBounded(Arc::new(Mutex::new(rx)))),
        ))
    }));
    m.insert("fungo".to_string(), Box::new(|args: &[Value]| {
        let inner = args.first().cloned().unwrap_or(Value::Hamna);
        match inner.try_into_send() {
            Some(sv) => Ok(Value::Tokeo(Ok(Box::new(Value::Fungo(Arc::new(value::FungoCell::new(sv))))))),
            None => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "fungo: thamani ya ndani haiwezi kuvuka nyuzi (Kasha_GC/Faili/Mkondo)".to_string(),
            ))))),
        }
    }));
}
