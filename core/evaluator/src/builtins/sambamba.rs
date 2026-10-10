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
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

use super::BuiltinFn;
use crate::value::{self, EvalError, Value};

static NEXT_HANDLE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn handles() -> &'static Mutex<HashMap<u64, JoinHandle<Result<(), String>>>> {
    static HANDLES: OnceLock<Mutex<HashMap<u64, JoinHandle<Result<(), String>>>>> = OnceLock::new();
    HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `tenda(kazi_jina, hoja...) -> Tokeo<Namba, Neno>` — runs `kazi_jina` (a module-level `kazi`
/// of `program`) on a new OS thread, on the same engine as the caller (bytecode and native code
/// for a bytecode program). Returns a handle id on success. The spawned function's own return
/// value is discarded (see module doc comment) — communicate results back via `njia`.
pub(crate) fn tenda(program: &crate::spawn::Shared, args: &[Value]) -> Result<Value, EvalError> {
    let kazi_name = super::arg_str(args, 0);
    let raw_args = args.get(1..).unwrap_or(&[]);
    let call_args: Vec<value::SendValue> = match raw_args.iter().map(Value::try_into_send).collect()
    {
        Some(v) => v,
        None => {
            return Ok(Value::kosa(
                "tenda: hoja ina thamani isiyoweza kuvuka nyuzi (Kasha_GC/Faili/Mkondo) — tumia njia/fungo badala yake".to_string(),
            ));
        }
    };
    if !program.has_kazi(&kazi_name) {
        return Ok(Value::kosa(format!("tenda: kazi haijulikani: {kazi_name}")));
    }
    let program = program.clone();
    let handle = std::thread::spawn(move || {
        let call_args: Vec<Value> = call_args
            .into_iter()
            .map(value::SendValue::into_value)
            .collect();
        program.with_caller(|call| {
            call(&kazi_name, call_args)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
    });
    let id = NEXT_HANDLE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    crate::sync::lock(handles()).insert(id, handle);
    Ok(Value::sawa(Value::Namba(id as f64)))
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "subiri_tenda".to_string(),
        Box::new(|args: &[Value]| {
            let id = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0) as u64;
            let handle = crate::sync::lock(handles()).remove(&id);
            // About to block: show what was printed so far.
            crate::platform::flush_stdout();
            if let (Some(h), true) = (&handle, crate::kazi_sawia::tasks_active()) {
                crate::kazi_sawia::wait_until(|| h.is_finished())?;
            }
            match handle {
                None => Ok(Value::kosa(format!(
                    "subiri_tenda: uzi haujulikani au tayari umesubiriwa: {id}"
                ))),
                Some(h) => match h.join() {
                    Ok(Ok(())) => Ok(Value::sawa(Value::Tupu)),
                    Ok(Err(msg)) => Ok(Value::kosa(msg)),
                    Err(_) => Ok(Value::kosa(
                        "subiri_tenda: uzi ulianguka (panic)".to_string(),
                    )),
                },
            }
        }),
    );
    // `subiri` (the keyword) lowers to this.
    m.insert(
        "__subiri".to_string(),
        Box::new(|args: &[Value]| match args.first() {
            Some(Value::Ahadi(t)) => crate::kazi_sawia::subiri(t),
            Some(other) => Ok(other.clone()),
            None => Ok(Value::Tupu),
        }),
    );
    m.insert(
        "subiri_zote".to_string(),
        Box::new(|args: &[Value]| {
            let tasks = ahadi_list(args.first(), "subiri_zote")?;
            let mut out = Vec::with_capacity(tasks.len());
            for t in &tasks {
                out.push(crate::kazi_sawia::subiri(t)?);
            }
            Ok(Value::list(out))
        }),
    );
    m.insert(
        "subiri_yoyote".to_string(),
        Box::new(|args: &[Value]| {
            let tasks = ahadi_list(args.first(), "subiri_yoyote")?;
            if tasks.is_empty() {
                return Err(EvalError::TypeErr("subiri_yoyote: orodha tupu".into()));
            }
            let (i, outcome) = crate::kazi_sawia::subiri_yoyote(&tasks)?;
            Ok(Value::Jozi(
                Box::new(Value::Namba(i as f64)),
                Box::new(outcome?),
            ))
        }),
    );
    m.insert(
        "muda_kikomo".to_string(),
        Box::new(|args: &[Value]| {
            let Some(Value::Ahadi(t)) = args.first() else {
                return Err(EvalError::TypeErr("muda_kikomo inahitaji Ahadi".into()));
            };
            let secs = value::as_f64(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or(0.0);
            Ok(match crate::kazi_sawia::subiri_kwa_muda(t, secs)? {
                Some(outcome) => Value::Chaguo(Some(Box::new(outcome?))),
                None => Value::Chaguo(None),
            })
        }),
    );
    m.insert(
        "ghairi".to_string(),
        Box::new(|args: &[Value]| {
            if let Some(Value::Ahadi(t)) = args.first() {
                t.cancel();
            }
            Ok(Value::Tupu)
        }),
    );
    m.insert(
        "njia".to_string(),
        Box::new(|_args: &[Value]| {
            let (tx, rx) = flume::unbounded::<value::SendValue>();
            Ok(Value::Jozi(
                Box::new(Value::NjiaTx(tx)),
                Box::new(Value::NjiaRx(rx)),
            ))
        }),
    );
    m.insert(
        "njia_na_kikomo".to_string(),
        Box::new(|args: &[Value]| {
            let kikomo = value::as_f64(args.first().unwrap_or(&Value::Hamna))
                .unwrap_or(0.0)
                .max(0.0) as usize;
            let (tx, rx) = flume::bounded::<value::SendValue>(kikomo);
            Ok(Value::Jozi(
                Box::new(Value::NjiaTxBounded(tx)),
                Box::new(Value::NjiaRxBounded(rx)),
            ))
        }),
    );
    m.insert(
        "fungo".to_string(),
        Box::new(|args: &[Value]| {
            let inner = args.first().cloned().unwrap_or(Value::Hamna);
            match inner.try_into_send() {
                Some(sv) => Ok(Value::sawa(Value::Fungo(Arc::new(value::FungoCell::new(
                    sv,
                ))))),
                None => Ok(Value::kosa(
                    "fungo: thamani ya ndani haiwezi kuvuka nyuzi (Kasha_GC/Faili/Mkondo)"
                        .to_string(),
                )),
            }
        }),
    );
}

/// The `Ahadi`s of an `Orodha<Ahadi<T>>` argument.
fn ahadi_list(
    v: Option<&Value>,
    name: &str,
) -> Result<Vec<std::rc::Rc<crate::kazi_sawia::Task>>, EvalError> {
    let Some(Value::Orodha(items)) = v else {
        return Err(EvalError::TypeErr(format!(
            "{name} inahitaji Orodha<Ahadi>"
        )));
    };
    items
        .iter()
        .map(|x| match x {
            Value::Ahadi(t) => Ok(t.clone()),
            _ => Err(EvalError::TypeErr(format!(
                "{name}: kila kipengele lazima kiwe Ahadi"
            ))),
        })
        .collect()
}
