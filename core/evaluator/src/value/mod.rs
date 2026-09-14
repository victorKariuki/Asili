//! Value model, errors, control flow, and numeric helpers.

mod json;
mod numeric;

use std::cell::{RefCell, UnsafeCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub use bigdecimal::BigDecimal;
pub use num_bigint::BigInt;

pub(crate) use numeric::{
    args_f64_2, arg_f64, assign_f64_op, as_char, as_f64, as_string, as_u64, big_numeric_binary_op,
    binary_cmp_neno, binary_f64, binary_f64_cmp, handle_loop_out, parse_number,
};

/// Hashable key for Kamusi. Only Neno, Namba, Ukweli, Herufi are allowed as map keys.
/// Namba uses f64::to_bits() for canonical hashing (NaN is supported).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapKey {
    Neno(String),
    Namba(u64),
    Ukweli(bool),
    Herufi(char),
}

impl MapKey {
    pub fn to_value(&self) -> Value {
        match self {
            MapKey::Neno(s) => Value::Neno(s.clone()),
            MapKey::Namba(b) => Value::Namba(f64::from_bits(*b)),
            MapKey::Ukweli(b) => Value::Ukweli(*b),
            MapKey::Herufi(c) => Value::Herufi(*c),
        }
    }

    pub fn try_from_value(v: &Value) -> Result<MapKey, EvalError> {
        match v {
            Value::Neno(s) => Ok(MapKey::Neno(s.clone())),
            Value::Namba(n) => Ok(MapKey::Namba(n.to_bits())),
            Value::Ukweli(b) => Ok(MapKey::Ukweli(*b)),
            Value::Herufi(c) => Ok(MapKey::Herufi(*c)),
            _ => Err(EvalError::TypeErr(
                "kamusi: ufunguo lazima uwe Neno, Namba, Ukweli au Herufi".into(),
            )),
        }
    }
}

// TODO(Phase III): Missing value variants from the spec (Seti landed, see below):
//   - Mfululizo(&[Value]) — slice/view into an Orodha without cloning (requires lifetime or Rc;
//     blocked on the borrow-checker lifetime-strategy decision, see
//     docs/design/phase3-self-hosting-borrow-checker-design.md)
//   - NambaKuu(BigInt) — arbitrary-precision integer (Namba_Kuu); needs the `num-bigint` crate
//   - NambaSahihi(BigDecimal) — arbitrary-precision decimal (Namba_Sahihi)
//   - FixedInt(i64, IntWidth) — Biti8/Biti32/Biti64/uBiti8/uBiti32/uBiti64 distinct from Namba
// Adding these requires updating all match arms in eval/expr.rs and the bytecode VM.
// Faili/Mkondo/Kumbukumbu<T> (resource handles) landed below — see docs/design/
// faili-mkondo-design.md.

#[derive(Clone)]
pub enum Value {
    Namba(f64),
    Neno(String),
    Ukweli(bool),
    Tupu,
    Hamna,
    Chaguo(Option<Box<Value>>),
    Tokeo(Result<Box<Value>, Box<Value>>),
    Orodha(Vec<Value>),
    Struct(String, Vec<(String, Value)>),
    Enum(String, String, Option<Box<Value>>), // enum_name, variant_name, optional_data
    Herufi(char),
    Jozi(Box<Value>, Box<Value>),
    Kamusi(HashMap<MapKey, Value>),
    /// Time: seconds since Unix epoch (majira module).
    Wakati(f64),
    /// Raw memory address (syscall, kiungo).
    Anuani(u64),
    /// Reference-counted shared wrapper (opt-in `leta kasha_gc`). Cloning a `Value` clones the
    /// `Rc` handle (cheap, shares the allocation) — use `.shirikisha()`/`kasha_gc_shiriki` to be
    /// explicit about that at the Asili level; `Value::clone()` alone does not increment beyond
    /// what `Rc::clone` already does.
    KashaGC(Rc<RefCell<Value>>),
    /// Weak reference to a `KashaGC<T>` (`kasha_gc_dhaifu`, downgrade). `Kasha_GC<T>` has no
    /// cycle collector — a reference cycle through it leaks permanently — so this is the
    /// user-level escape hatch: hold a `Dhaifu` in the back-pointer of a cycle-prone structure
    /// (e.g. a child referencing its parent) and `.imarisha()` (upgrade, `Weak::upgrade`) only
    /// when actually needed, so the cycle's strong-count can still reach zero.
    KashaGCDhaifu(std::rc::Weak<RefCell<Value>>),
    /// File handle. Closed on drop via `FailiHandle`'s own `Drop` impl — this fires whether the
    /// value is removed by explicit `tupa`, by `Env::drop`, or by `Env::pop_scope`'s bare
    /// `HashMap` teardown at block/function exit (which does not call `Env::drop` at all), since
    /// all three paths end in ordinary Rust value drop. Double-close is a safe no-op (`Option`
    /// is `None` after the first close).
    Faili(Rc<RefCell<FailiHandle>>),
    /// TCP stream handle. Same drop semantics as `Faili`.
    Mkondo(Rc<RefCell<MkondoHandle>>),
    /// TCP listening socket (`mkondo_sikiliza`). Unlike `Mkondo`/`Faili`, this genuinely crosses
    /// thread boundaries by design — `mkondo_tumikia`'s worker pool has every worker thread
    /// calling `.accept()` on the same listener concurrently (safe: `TcpListener::accept` takes
    /// `&self`, and the OS itself serializes concurrent accepts on one socket — no lock needed
    /// on the hot path) — so this is `Arc<TcpListener>`, not `Rc`, the one justified exception
    /// in the Faili/Mkondo family for the same reason `NjiaTx`/`NjiaRx`/`Fungo` are `Arc`
    /// instead of `Rc`. No explicit `.funga()`: closing happens only via every `Arc` clone
    /// (main handle + every worker thread's copy) dropping, which is what actually stopping a
    /// server means under the "blocks forever" `mkondo_tumikia` model this plan adopted — an
    /// explicit close while workers still hold clones would be a footgun, not a useful control.
    MkondoSikilizaji(Arc<std::net::TcpListener>),
    /// A loaded TLS server certificate/key pair, ready to hand to `mkondo_tumikia`
    /// (`tls_sanidi`). `rustls::ServerConfig` is `Send + Sync` by design — every rustls consumer
    /// `Arc`-shares it across connections — so this fits the worker pool's existing
    /// `Arc`-sharing model directly, the same way `MkondoSikilizaji` already shares one listener
    /// across every worker thread.
    #[cfg(not(target_arch = "wasm32"))]
    TlsUsanidi(Arc<rustls::ServerConfig>),
    /// Heap-allocated box owning a value of type T; no OS resource, plain owning indirection —
    /// existing Box/Value drop and clone semantics already suffice, no special handling needed.
    Kumbukumbu(Box<Value>),
    /// Ordered-by-nothing set (Seti<T>); reuses the MapKey hashable-key type Kamusi already
    /// uses. Iteration order is HashSet's (unspecified), same tradeoff Kamusi already accepts.
    Seti(HashSet<MapKey>),
    /// Arbitrary-precision integer (Namba_Kuu). No literal syntax — constructed only via
    /// `namba_kuu_kutoka(neno)` parsing a decimal-digit string, or infallibly cast from `Namba`.
    NambaKuu(BigInt),
    /// Arbitrary-precision decimal (Namba_Sahihi). Same construction story as NambaKuu.
    NambaSahihi(BigDecimal),
    /// Channel sender half (njia). Deliberately `Arc<Mutex<_>>` over `SendValue`, not
    /// `Rc<RefCell<Value>>` like every other interior-mutable Value variant — a channel's
    /// entire purpose is crossing the thread boundary `tenda` spawns, so it's the one justified
    /// exception to this codebase's otherwise-single-threaded `Rc` convention. Carries
    /// `SendValue`, not `Value`: `Arc<Mutex<Value>>` would NOT actually be `Send` (`Mutex<T>`'s
    /// own `Sync` impl requires `T: Send`, and `Value` isn't unconditionally `Send` — the
    /// `Arc<Mutex<_>>` wrapper alone doesn't fix that, it just moves the requirement one level
    /// up), so the payload type itself has to be the already-restricted one.
    NjiaTx(Arc<Mutex<std::sync::mpsc::Sender<SendValue>>>),
    /// Channel receiver half (njia). Same `SendValue`-payload reasoning as `NjiaTx`.
    NjiaRx(Arc<Mutex<std::sync::mpsc::Receiver<SendValue>>>),
    /// Bounded channel sender half (`njia_na_kikomo`) — `mpsc::SyncSender`, a distinct Rust type
    /// from `mpsc::Sender` (hence its own `Value` variant rather than an internal enum inside
    /// `NjiaTx`), whose `.send()` blocks once the bound is full instead of growing memory
    /// without limit the way the unbounded `njia()` channel does. This is an additive
    /// constructor alongside `njia()`, not a change to its existing behavior.
    NjiaTxBounded(Arc<Mutex<std::sync::mpsc::SyncSender<SendValue>>>),
    /// Receiver half for a bounded channel. `mpsc::sync_channel` returns a plain
    /// `mpsc::Receiver` (identical to the unbounded case) — reuses `NjiaRx`'s exact payload
    /// type, so no separate receiver variant is needed, only a separate sender one.
    NjiaRxBounded(Arc<Mutex<std::sync::mpsc::Receiver<SendValue>>>),
    /// Mutex (fungo) — protects a shared value across tenda-spawned threads with **explicit**
    /// `.funga()`/`.fungua()` (lock/unlock), not a Rust-style scoped guard (Asili has no
    /// closures to scope a critical section with). Holds `SendValue` for the same reason
    /// `NjiaTx`/`NjiaRx` do, not `Kasha_GC<T>`'s `Rc<RefCell<Value>>>`. See `FungoCell`.
    Fungo(Arc<FungoCell>),
}

/// Backs `Value::Fungo`. `std::sync::Mutex`'s guard is scoped to a Rust lexical block and can't
/// be held across two separate Asili-level calls (`.funga()` then, later, `.fungua()`) — this
/// wraps `parking_lot`'s raw-mutex primitives instead, which are explicitly designed to support
/// exactly that "lock now, unlock later, possibly from different call sites" protocol.
pub struct FungoCell {
    raw: parking_lot::RawMutex,
    data: UnsafeCell<SendValue>,
}

// SAFETY: `raw` (parking_lot::RawMutex) provides the actual mutual exclusion guaranteeing only
// one thread accesses `data` at a time between a matched .funga()/.fungua() pair — this is
// exactly the same safety argument `lock_api::Mutex` itself relies on internally, just without
// a Rust-lifetime-scoped guard object (which is what a raw mutex is explicitly for). `SendValue`
// being `Send` (never `Sync` is asserted — `FungoCell` is accessed only through `Arc`, and its
// own locking is what provides safe concurrent access, not `SendValue`'s own `Sync`-ness) is
// what makes this sound to share across threads.
unsafe impl Send for FungoCell {}
unsafe impl Sync for FungoCell {}

impl FungoCell {
    pub(crate) fn new(v: SendValue) -> Self {
        use lock_api::RawMutex as _;
        FungoCell { raw: parking_lot::RawMutex::INIT, data: UnsafeCell::new(v) }
    }

    /// Blocks until the lock is acquired. Must be paired with exactly one later `unlock()` from
    /// the same logical "critical section" — calling `read`/`write` without holding the lock,
    /// or calling `unlock()` without a prior `lock()`, is a logic error at the Asili level
    /// (surfaced as a `Tokeo(Kosa(...))`, not undefined behavior — see `sambamba.rs`'s
    /// `.funga()`/`.fungua()`/`.pata()`/`.weka()` wrappers, which track lock state explicitly
    /// rather than trusting the caller).
    pub(crate) fn lock(&self) {
        lock_api::RawMutex::lock(&self.raw);
    }

    /// # Safety
    /// The caller must currently hold the lock (via a prior successful `lock()` on this same
    /// `FungoCell`) and must not read/write `data` after calling this until locking again.
    pub(crate) unsafe fn unlock(&self) {
        lock_api::RawMutex::unlock(&self.raw);
    }

    pub(crate) fn try_lock(&self) -> bool {
        lock_api::RawMutex::try_lock(&self.raw)
    }

    /// # Safety
    /// The caller must currently hold the lock.
    pub(crate) unsafe fn read(&self) -> SendValue {
        (*self.data.get()).clone()
    }

    /// # Safety
    /// The caller must currently hold the lock.
    pub(crate) unsafe fn write(&self, v: SendValue) {
        *self.data.get() = v;
    }
}

/// A `Value` restricted to the subset of variants that are unconditionally `Send` — used
/// anywhere a value needs to cross a real OS thread boundary (`tenda`'s spawned-function
/// arguments/channel payloads). `Value` as a whole cannot be made `Send` without `unsafe` (some
/// variants — `KashaGC`/`Faili`/`Mkondo`, and transitively `Kumbukumbu` since it can box any of
/// them — hold `Rc`, which is deliberately not `Send`); rather than assert around that with an
/// `unsafe impl Send` wrapper (sound only as long as every future `Rc`-based `Value` variant
/// remembers to be excluded — a real, recurring risk, not a one-time cost), this type makes the
/// restriction structural and checked by the compiler instead of by convention. Build one via
/// `Value::try_into_send()`; convert back via `.into_value()` (infallible — every `SendValue`
/// variant maps to exactly one `Value` variant).
#[derive(Clone)]
pub enum SendValue {
    Namba(f64),
    Neno(String),
    Ukweli(bool),
    Tupu,
    Hamna,
    Herufi(char),
    Wakati(f64),
    Anuani(u64),
    NambaKuu(BigInt),
    NambaSahihi(BigDecimal),
    Chaguo(Option<Box<SendValue>>),
    Tokeo(Result<Box<SendValue>, Box<SendValue>>),
    Orodha(Vec<SendValue>),
    Jozi(Box<SendValue>, Box<SendValue>),
    Kamusi(HashMap<MapKey, SendValue>),
    Seti(HashSet<MapKey>),
    Struct(String, Vec<(String, SendValue)>),
    Enum(String, String, Option<Box<SendValue>>),
    /// `Value`'s `NjiaTx`/`NjiaRx`/`Fungo` already carry `SendValue` payloads (see their own
    /// doc comments on `Value`), so these clone the `Arc` handle directly — no conversion.
    NjiaTx(Arc<Mutex<std::sync::mpsc::Sender<SendValue>>>),
    NjiaRx(Arc<Mutex<std::sync::mpsc::Receiver<SendValue>>>),
    NjiaTxBounded(Arc<Mutex<std::sync::mpsc::SyncSender<SendValue>>>),
    NjiaRxBounded(Arc<Mutex<std::sync::mpsc::Receiver<SendValue>>>),
    Fungo(Arc<FungoCell>),
}

impl Value {
    /// `None` if `self` is (or transitively contains) a non-`Send` variant
    /// (`KashaGC`/`Faili`/`Mkondo`, or a `Kumbukumbu` boxing one).
    pub fn try_into_send(&self) -> Option<SendValue> {
        Some(match self {
            Value::Namba(n) => SendValue::Namba(*n),
            Value::Neno(s) => SendValue::Neno(s.clone()),
            Value::Ukweli(b) => SendValue::Ukweli(*b),
            Value::Tupu => SendValue::Tupu,
            Value::Hamna => SendValue::Hamna,
            Value::Herufi(c) => SendValue::Herufi(*c),
            Value::Wakati(s) => SendValue::Wakati(*s),
            Value::Anuani(a) => SendValue::Anuani(*a),
            Value::NambaKuu(n) => SendValue::NambaKuu(n.clone()),
            Value::NambaSahihi(n) => SendValue::NambaSahihi(n.clone()),
            Value::Chaguo(opt) => SendValue::Chaguo(match opt {
                Some(v) => Some(Box::new(v.try_into_send()?)),
                None => None,
            }),
            Value::Tokeo(res) => SendValue::Tokeo(match res {
                Ok(v) => Ok(Box::new(v.try_into_send()?)),
                Err(v) => Err(Box::new(v.try_into_send()?)),
            }),
            Value::Orodha(items) => {
                SendValue::Orodha(items.iter().map(Value::try_into_send).collect::<Option<_>>()?)
            }
            Value::Jozi(a, b) => SendValue::Jozi(Box::new(a.try_into_send()?), Box::new(b.try_into_send()?)),
            Value::Kamusi(m) => {
                let mut out = HashMap::with_capacity(m.len());
                for (k, v) in m {
                    out.insert(k.clone(), v.try_into_send()?);
                }
                SendValue::Kamusi(out)
            }
            Value::Seti(s) => SendValue::Seti(s.clone()),
            Value::Struct(name, fields) => {
                let mut out = Vec::with_capacity(fields.len());
                for (fname, v) in fields {
                    out.push((fname.clone(), v.try_into_send()?));
                }
                SendValue::Struct(name.clone(), out)
            }
            Value::Enum(en, vn, data) => SendValue::Enum(
                en.clone(),
                vn.clone(),
                match data {
                    Some(v) => Some(Box::new(v.try_into_send()?)),
                    None => None,
                },
            ),
            Value::NjiaTx(tx) => SendValue::NjiaTx(Arc::clone(tx)),
            Value::NjiaRx(rx) => SendValue::NjiaRx(Arc::clone(rx)),
            Value::NjiaTxBounded(tx) => SendValue::NjiaTxBounded(Arc::clone(tx)),
            Value::NjiaRxBounded(rx) => SendValue::NjiaRxBounded(Arc::clone(rx)),
            Value::Fungo(cell) => SendValue::Fungo(Arc::clone(cell)),
            // MkondoSikilizaji (Arc<TcpListener>) and TlsUsanidi (Arc<rustls::ServerConfig>) are
            // technically Send-safe on their own, but mkondo_tumikia's worker pool spawns and
            // manages its own threads directly rather than routing through tenda/SendValue —
            // excluded here since nothing in this design needs either to cross that specific
            // boundary; revisit if that changes.
            #[cfg(not(target_arch = "wasm32"))]
            Value::TlsUsanidi(_) => return None,
            Value::KashaGC(_) | Value::KashaGCDhaifu(_) | Value::Faili(_) | Value::Mkondo(_) | Value::MkondoSikilizaji(_) | Value::Kumbukumbu(_) => return None,
        })
    }
}

impl SendValue {
    pub fn into_value(self) -> Value {
        match self {
            SendValue::Namba(n) => Value::Namba(n),
            SendValue::Neno(s) => Value::Neno(s),
            SendValue::Ukweli(b) => Value::Ukweli(b),
            SendValue::Tupu => Value::Tupu,
            SendValue::Hamna => Value::Hamna,
            SendValue::Herufi(c) => Value::Herufi(c),
            SendValue::Wakati(s) => Value::Wakati(s),
            SendValue::Anuani(a) => Value::Anuani(a),
            SendValue::NambaKuu(n) => Value::NambaKuu(n),
            SendValue::NambaSahihi(n) => Value::NambaSahihi(n),
            SendValue::Chaguo(opt) => Value::Chaguo(opt.map(|v| Box::new(v.into_value()))),
            SendValue::Tokeo(res) => Value::Tokeo(match res {
                Ok(v) => Ok(Box::new(v.into_value())),
                Err(v) => Err(Box::new(v.into_value())),
            }),
            SendValue::Orodha(items) => Value::Orodha(items.into_iter().map(SendValue::into_value).collect()),
            SendValue::Jozi(a, b) => Value::Jozi(Box::new(a.into_value()), Box::new(b.into_value())),
            SendValue::Kamusi(m) => Value::Kamusi(m.into_iter().map(|(k, v)| (k, v.into_value())).collect()),
            SendValue::Seti(s) => Value::Seti(s),
            SendValue::Struct(name, fields) => {
                Value::Struct(name, fields.into_iter().map(|(n, v)| (n, v.into_value())).collect())
            }
            SendValue::Enum(en, vn, data) => Value::Enum(en, vn, data.map(|v| Box::new(v.into_value()))),
            SendValue::NjiaTx(tx) => Value::NjiaTx(tx),
            SendValue::NjiaRx(rx) => Value::NjiaRx(rx),
            SendValue::NjiaTxBounded(tx) => Value::NjiaTxBounded(tx),
            SendValue::NjiaRxBounded(rx) => Value::NjiaRxBounded(rx),
            SendValue::Fungo(cell) => Value::Fungo(cell),
        }
    }
}

/// Owns an open file; `Drop` closes it exactly once regardless of which path (tupa, Env::drop,
/// or ordinary scope-exit) removed the last `Value` referencing it.
pub struct FailiHandle(pub Option<std::fs::File>);

impl Drop for FailiHandle {
    fn drop(&mut self) {
        // Taking + dropping the File is the close; nothing else to do. Explicit for clarity.
        let _ = self.0.take();
    }
}

/// The backing stream a `Mkondo` handle wraps — plain TCP, or a TLS session negotiated over TCP.
/// Distinguishing these as an enum (rather than a second `Value`/`MkondoHandle` type for TLS)
/// means `.soma()`/`.andika()`/`.funga()` (`eval/expr.rs`) stay completely unchanged: they call
/// through `MkondoStream`'s own `Read`/`Write` impls below, which dispatch to whichever variant
/// is active. `mkondo_unganisha`'s plaintext client path constructs `Wazi`; `mkondo_tumikia`'s
/// TLS branch (when a `TlsUsanidi` is passed) constructs `Salama` after a successful handshake.
///
/// `Salama`'s payload is boxed — `rustls::StreamOwned` is large relative to a bare `TcpStream`,
/// and boxing keeps the common (plaintext) case of `Value::Mkondo`'s `Rc<RefCell<MkondoHandle>>`
/// from paying that size cost when TLS isn't in use at all.
pub enum MkondoStream {
    Wazi(std::net::TcpStream),
    #[cfg(not(target_arch = "wasm32"))]
    Salama(Box<rustls::StreamOwned<rustls::ServerConnection, std::net::TcpStream>>),
}

impl std::io::Read for MkondoStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            MkondoStream::Wazi(s) => s.read(buf),
            #[cfg(not(target_arch = "wasm32"))]
            MkondoStream::Salama(s) => s.read(buf),
        }
    }
}

impl std::io::Write for MkondoStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            MkondoStream::Wazi(s) => s.write(buf),
            #[cfg(not(target_arch = "wasm32"))]
            MkondoStream::Salama(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            MkondoStream::Wazi(s) => s.flush(),
            #[cfg(not(target_arch = "wasm32"))]
            MkondoStream::Salama(s) => s.flush(),
        }
    }
}

impl Drop for MkondoStream {
    fn drop(&mut self) {
        // TLS requires a protocol-level `close_notify` before the underlying TCP socket closes
        // — a bare TCP close (what happens for free on `Wazi`) is indistinguishable to the peer
        // from a truncation attack, and rustls correctly treats it as an error
        // ("peer closed connection without sending TLS close_notify") rather than a clean EOF.
        // Without this, every `.funga()`/scope-exit/drop of a TLS `Mkondo` handle would make the
        // *peer's* next read fail even though every application byte arrived correctly.
        #[cfg(not(target_arch = "wasm32"))]
        if let MkondoStream::Salama(s) = self {
            use std::io::Write as _;
            s.conn.send_close_notify();
            let _ = s.flush();
        }
    }
}

/// Owns an open TCP stream (plain or TLS); same drop discipline as `FailiHandle`.
pub struct MkondoHandle(pub Option<MkondoStream>);

impl Drop for MkondoHandle {
    fn drop(&mut self) {
        let _ = self.0.take();
    }
}

// Manual Debug impl (not #[derive]) so raw/REPL output uses Asili's own variant names —
// `Tokeo`/`Chaguo` wrap Rust's `Result`/`Option`, whose derived Debug would otherwise print
// the Rust-side `Ok(..)`/`Err(..)`/`Some(..)` literally instead of `Sawa(..)`/`Kosa(..)`/
// `Kuna(..)` (see `core/parser/src/parse.rs`'s `standard_enums()` for the canonical names).
impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Namba(n) => f.debug_tuple("Namba").field(n).finish(),
            Value::Neno(s) => f.debug_tuple("Neno").field(s).finish(),
            Value::Ukweli(b) => f.debug_tuple("Ukweli").field(b).finish(),
            Value::Tupu => write!(f, "Tupu"),
            Value::Hamna => write!(f, "Hamna"),
            Value::Chaguo(Some(v)) => write!(f, "Chaguo(Kuna({v:?}))"),
            Value::Chaguo(None) => write!(f, "Chaguo(Hamna)"),
            Value::Tokeo(Ok(v)) => write!(f, "Tokeo(Sawa({v:?}))"),
            Value::Tokeo(Err(e)) => write!(f, "Tokeo(Kosa({e:?}))"),
            Value::Orodha(items) => f.debug_tuple("Orodha").field(items).finish(),
            Value::Struct(name, fields) => f.debug_tuple("Struct").field(name).field(fields).finish(),
            Value::Enum(en, vn, data) => f.debug_tuple("Enum").field(en).field(vn).field(data).finish(),
            Value::Herufi(c) => f.debug_tuple("Herufi").field(c).finish(),
            Value::Jozi(a, b) => f.debug_tuple("Jozi").field(a).field(b).finish(),
            Value::Kamusi(m) => f.debug_tuple("Kamusi").field(m).finish(),
            Value::Wakati(s) => f.debug_tuple("Wakati").field(s).finish(),
            Value::Anuani(a) => f.debug_tuple("Anuani").field(a).finish(),
            Value::KashaGC(cell) => f.debug_tuple("KashaGC").field(cell).finish(),
            Value::KashaGCDhaifu(weak) => {
                write!(f, "KashaGCDhaifu({})", if weak.strong_count() > 0 { "hai" } else { "imekufa" })
            }
            Value::Faili(cell) => {
                let open = cell.borrow().0.is_some();
                write!(f, "Faili({})", if open { "wazi" } else { "imefungwa" })
            }
            Value::Mkondo(cell) => {
                let open = cell.borrow().0.is_some();
                write!(f, "Mkondo({})", if open { "wazi" } else { "imefungwa" })
            }
            Value::MkondoSikilizaji(listener) => {
                write!(f, "MkondoSikilizaji({:?})", listener.local_addr())
            }
            #[cfg(not(target_arch = "wasm32"))]
            Value::TlsUsanidi(_) => write!(f, "TlsUsanidi"),
            Value::Kumbukumbu(v) => f.debug_tuple("Kumbukumbu").field(v).finish(),
            Value::Seti(s) => f.debug_tuple("Seti").field(s).finish(),
            Value::NambaKuu(n) => f.debug_tuple("NambaKuu").field(n).finish(),
            Value::NambaSahihi(n) => f.debug_tuple("NambaSahihi").field(n).finish(),
            Value::NjiaTx(_) => write!(f, "NjiaTx"),
            Value::NjiaRx(_) => write!(f, "NjiaRx"),
            Value::NjiaTxBounded(_) => write!(f, "NjiaTxBounded"),
            Value::NjiaRxBounded(_) => write!(f, "NjiaRxBounded"),
            Value::Fungo(cell) => {
                if cell.try_lock() {
                    // SAFETY: try_lock() just succeeded, so this call holds the lock.
                    let v = unsafe { cell.read() };
                    // SAFETY: still holding the same lock acquired immediately above.
                    unsafe { cell.unlock() };
                    write!(f, "Fungo({:?})", v.into_value())
                } else {
                    write!(f, "Fungo(<imefungwa>)")
                }
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Namba(a), Value::Namba(b)) => a == b,
            (Value::Neno(a), Value::Neno(b)) => a == b,
            (Value::Ukweli(a), Value::Ukweli(b)) => a == b,
            (Value::Tupu, Value::Tupu) => true,
            (Value::Hamna, Value::Hamna) => true,
            (Value::Chaguo(a), Value::Chaguo(b)) => a == b,
            (Value::Tokeo(a), Value::Tokeo(b)) => a == b,
            (Value::Orodha(a), Value::Orodha(b)) => a == b,
            (Value::Struct(an, af), Value::Struct(bn, bf)) => an == bn && af == bf,
            (Value::Enum(an, av, ad), Value::Enum(bn, bv, bd)) => an == bn && av == bv && ad == bd,
            (Value::Herufi(a), Value::Herufi(b)) => a == b,
            (Value::Jozi(a1, a2), Value::Jozi(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Kamusi(a), Value::Kamusi(b)) => a == b,
            (Value::Wakati(a), Value::Wakati(b)) => a == b,
            (Value::Anuani(a), Value::Anuani(b)) => a == b,
            // Identity, not structural equality: comparing contents via RefCell::eq would panic
            // if either handle is currently mutably borrowed. Two handles are "equal" iff they
            // share the same allocation (the same underlying `weka` binding's shared cell).
            (Value::KashaGC(a), Value::KashaGC(b)) => Rc::ptr_eq(a, b),
            (Value::KashaGCDhaifu(a), Value::KashaGCDhaifu(b)) => a.ptr_eq(b),
            // Identity, not structural: a file/stream handle is "equal" iff it's the same
            // underlying OS resource, not two separately-opened handles to the same path.
            (Value::Faili(a), Value::Faili(b)) => Rc::ptr_eq(a, b),
            (Value::Mkondo(a), Value::Mkondo(b)) => Rc::ptr_eq(a, b),
            (Value::MkondoSikilizaji(a), Value::MkondoSikilizaji(b)) => Arc::ptr_eq(a, b),
            (Value::Kumbukumbu(a), Value::Kumbukumbu(b)) => a == b,
            (Value::Seti(a), Value::Seti(b)) => a == b,
            (Value::NambaKuu(a), Value::NambaKuu(b)) => a == b,
            (Value::NambaSahihi(a), Value::NambaSahihi(b)) => a == b,
            (Value::NjiaTx(a), Value::NjiaTx(b)) => Arc::ptr_eq(a, b),
            (Value::NjiaRx(a), Value::NjiaRx(b)) => Arc::ptr_eq(a, b),
            (Value::NjiaTxBounded(a), Value::NjiaTxBounded(b)) => Arc::ptr_eq(a, b),
            (Value::NjiaRxBounded(a), Value::NjiaRxBounded(b)) => Arc::ptr_eq(a, b),
            (Value::Fungo(a), Value::Fungo(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// Transport-agnostic error classification for [`EvalError::Coded`]. Deliberately generic names
/// (not `BadRequest`/404-as-a-name) rather than HTTP-specific ones — `EvalError` is a
/// core-evaluator type used by the REPL/CLI too, not only a future HTTP server layer. A future
/// HTTP layer maps these to status codes itself (`BadInput` -> 400, `NotFound` -> 404,
/// `Conflict` -> 409, `Unavailable` -> 503, `Internal` -> 500); that mapping lives there, not
/// here, keeping the evaluator itself transport-agnostic. See docs/spec/08-resolved-decisions.md
/// for why this is additive (a new variant) rather than a restructure of the existing ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The caller supplied invalid/malformed input (bad arguments, invalid JSON, ...).
    BadInput,
    /// A referenced resource/entry doesn't exist.
    NotFound,
    /// The request conflicts with existing state.
    Conflict,
    /// An unexpected internal fault, or a fault this codebase can't yet classify more precisely.
    Internal,
    /// A dependency or resource is temporarily unavailable (would-block, capacity, etc.).
    Unavailable,
}

#[derive(Debug)]
pub enum EvalError {
    Panic(String),
    UndefinedVar(String),
    TypeErr(String),
    DivByZero,
    /// Propagate: ? on Tokeo(Err) — return this value from the current function.
    Propagate(Value),
    Unknown(String),
    /// A classified error carrying an explicit [`ErrorKind`], for callers (new builtins going
    /// forward — the JSON codec, a future HTTP listener) that want a status-code-mappable error
    /// without inventing a new `EvalError` variant per error site. Existing variants
    /// (`TypeErr`/`UndefinedVar`/`DivByZero`/`Panic`) are intentionally left as-is rather than
    /// migrated — see `From<&EvalError> for ErrorKind` for the conservative fallback every
    /// pre-existing error site gets for free.
    Coded { kind: ErrorKind, message: String },
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::Panic(m) => write!(f, "paparika: {m}"),
            EvalError::UndefinedVar(n) => write!(f, "jina '{n}' halijulikani"),
            EvalError::TypeErr(m) => write!(f, "aina: {m}"),
            EvalError::DivByZero => write!(f, "gawio kwa sifuri"),
            EvalError::Propagate(v) => write!(f, "KOSA: {v:?}"),
            EvalError::Unknown(m) => write!(f, "{m}"),
            EvalError::Coded { message, .. } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for EvalError {}

/// Conservative fallback: every pre-existing `EvalError` variant maps to `Internal` (the safest
/// default status a caller can assume when it doesn't know better), so nothing already in the
/// codebase needs to change to get a status code out of a future HTTP layer. `Coded` errors
/// report their own explicit kind, since that's the whole point of constructing one.
impl From<&EvalError> for ErrorKind {
    fn from(err: &EvalError) -> ErrorKind {
        match err {
            EvalError::Coded { kind, .. } => *kind,
            EvalError::Panic(_)
            | EvalError::UndefinedVar(_)
            | EvalError::TypeErr(_)
            | EvalError::DivByZero
            | EvalError::Propagate(_)
            | EvalError::Unknown(_) => ErrorKind::Internal,
        }
    }
}

#[derive(Debug)]
pub enum EvalOut {
    Return(Value),
    Break(Option<String>),
    Continue(Option<String>),
    Next,
}

#[derive(Debug)]
pub(crate) enum LoopAction {
    Continue,
    Break,
    Propagate(EvalOut),
}

#[cfg(test)]
mod error_kind_tests {
    use super::{ErrorKind, EvalError};

    #[test]
    fn coded_error_reports_its_own_kind() {
        let err = EvalError::Coded { kind: ErrorKind::NotFound, message: "haipo".into() };
        assert_eq!(ErrorKind::from(&err), ErrorKind::NotFound);
    }

    #[test]
    fn coded_error_display_shows_message_only() {
        let err = EvalError::Coded { kind: ErrorKind::BadInput, message: "data mbaya".into() };
        assert_eq!(err.to_string(), "data mbaya");
    }

    #[test]
    fn every_pre_existing_variant_falls_back_to_internal() {
        assert_eq!(ErrorKind::from(&EvalError::Panic("x".into())), ErrorKind::Internal);
        assert_eq!(ErrorKind::from(&EvalError::UndefinedVar("x".into())), ErrorKind::Internal);
        assert_eq!(ErrorKind::from(&EvalError::TypeErr("x".into())), ErrorKind::Internal);
        assert_eq!(ErrorKind::from(&EvalError::DivByZero), ErrorKind::Internal);
        assert_eq!(ErrorKind::from(&EvalError::Unknown("x".into())), ErrorKind::Internal);
        assert_eq!(
            ErrorKind::from(&EvalError::Propagate(super::Value::Tupu)),
            ErrorKind::Internal
        );
    }
}
