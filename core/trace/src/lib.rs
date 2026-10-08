//! Pata-Trace: one tracing API for the Asili compiler and runtime.
//!
//! The compiler and runtime only ever call [`emit`] and [`enter`] (a span that ends when dropped).
//! Where the events go is decided once, at start-up, by installing a [`Sink`]:
//!
//! * [`TreeSink`] — an indented, readable tree with Swahili labels (`pata ... --fuatilia`);
//! * [`JsonSink`] — one JSON object per line, spans and events in OpenTelemetry's shape
//!   (`traceId`/`spanId`/`parentSpanId`, `startTimeUnixNano`/`endTimeUnixNano`, `attributes`),
//!   for log and tracing back ends;
//! * [`BinarySink`] — fixed 4-byte frames (event id, nesting depth, a 16-bit payload: the source
//!   line), no text at all; [`decode`] turns a recording back into the Swahili tree.
//!
//! Tracing is off unless a sink is installed, and then costs one relaxed atomic load per hook
//! ([`on`]). Native code carries no hooks: calls between native functions are not traced, only
//! what passes through the runtime.

use std::cell::Cell;
use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// What happened: the one-byte id every output format shares.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tukio {
    /// A span starts: a `kazi` call, a block, a build phase.
    Ingia = 0x01,
    /// The innermost span ends.
    Toka = 0x02,
    /// A binding is created (`weka`, `thabiti`).
    Kigeuzi = 0x03,
    /// An arithmetic operation (reserved: not emitted, it would drown every other event).
    Hesabu = 0x04,
    /// A builtin (the program's view of the system) is called.
    MwitoMfumo = 0x05,
    /// A runtime error leaves a `kazi`.
    Kosa = 0x06,
    /// A build phase (parsing, checking, compiling) starts: a span, closed by [`Tukio::Toka`].
    Hatua = 0x07,
    /// The parser skipped ahead after a syntax error.
    Urejeshaji = 0x08,
}

impl Tukio {
    const ALL: [Tukio; 8] = [
        Tukio::Ingia,
        Tukio::Toka,
        Tukio::Kigeuzi,
        Tukio::Hesabu,
        Tukio::MwitoMfumo,
        Tukio::Kosa,
        Tukio::Hatua,
        Tukio::Urejeshaji,
    ];

    pub fn from_byte(b: u8) -> Option<Tukio> {
        Self::ALL.into_iter().find(|t| *t as u8 == b)
    }

    /// What the event means, in Swahili (the readable tree and the decoder).
    pub fn maelezo(self) -> &'static str {
        match self {
            Tukio::Ingia => "kuingia",
            Tukio::Toka => "kutoka",
            Tukio::Kigeuzi => "kutenga nafasi ya kigeuzi",
            Tukio::Hesabu => "operesheni ya hesabu",
            Tukio::MwitoMfumo => "mwito wa mfumo",
            Tukio::Kosa => "kosa",
            Tukio::Hatua => "hatua ya ujenzi",
            Tukio::Urejeshaji => "urejeshaji baada ya kosa la sintaksia",
        }
    }

    /// A stable machine name (the JSON output).
    pub fn jina(self) -> &'static str {
        match self {
            Tukio::Ingia => "ingia",
            Tukio::Toka => "toka",
            Tukio::Kigeuzi => "kigeuzi",
            Tukio::Hesabu => "hesabu",
            Tukio::MwitoMfumo => "mwito_mfumo",
            Tukio::Kosa => "kosa",
            Tukio::Hatua => "hatua",
            Tukio::Urejeshaji => "urejeshaji",
        }
    }
}

/// One event as a sink receives it.
#[derive(Clone, Copy, Debug)]
pub struct Event<'a> {
    pub tukio: Tukio,
    /// Spans open on this thread when it happened (a span's own `Ingia`/`Toka` carry the depth
    /// outside it).
    pub depth: u16,
    /// A `kazi`, binding, builtin or phase name, or an error message.
    pub name: &'a str,
    /// Source line, or 0.
    pub line: u32,
    /// Small per-process thread number (1 is the first thread that traced).
    pub thread: u64,
    /// Nanoseconds since tracing started.
    pub time_ns: u64,
}

/// Where events go.
pub trait Sink: Send {
    fn record(&mut self, event: &Event<'_>);
    fn flush(&mut self) {}
}

static ON: AtomicBool = AtomicBool::new(false);
static SINK: Mutex<Option<Box<dyn Sink>>> = Mutex::new(None);
static START: OnceLock<Instant> = OnceLock::new();
static NEXT_THREAD: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static DEPTH: Cell<u16> = const { Cell::new(0) };
    static THREAD: Cell<u64> = const { Cell::new(0) };
}

/// Whether tracing is on: the one check every hook makes first.
#[inline(always)]
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Send every following event to `sink`.
pub fn install(sink: Box<dyn Sink>) {
    START.get_or_init(Instant::now);
    *SINK.lock().unwrap_or_else(|e| e.into_inner()) = Some(sink);
    ON.store(true, Ordering::Relaxed);
}

/// Stop tracing and flush the sink. Call before the process exits.
pub fn finish() {
    ON.store(false, Ordering::Relaxed);
    if let Some(mut sink) = SINK.lock().unwrap_or_else(|e| e.into_inner()).take() {
        sink.flush();
    }
}

fn thread_number() -> u64 {
    THREAD.with(|t| {
        if t.get() == 0 {
            t.set(NEXT_THREAD.fetch_add(1, Ordering::Relaxed));
        }
        t.get()
    })
}

fn record(tukio: Tukio, depth: u16, name: &str, line: u32) {
    let time_ns = START.get().map_or(0, |s| s.elapsed().as_nanos() as u64);
    let event = Event {
        tukio,
        depth,
        name,
        line,
        thread: thread_number(),
        time_ns,
    };
    if let Some(sink) = SINK.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        sink.record(&event);
    }
}

/// Record a single event at the current depth (nothing when tracing is off).
#[inline]
pub fn emit(tukio: Tukio, name: &str, line: u32) {
    if on() {
        record(tukio, DEPTH.with(Cell::get), name, line);
    }
}

/// Open a span; it ends when the returned guard is dropped (nothing when tracing is off).
#[inline]
pub fn enter(name: &str, line: u32) -> Span {
    open(Tukio::Ingia, name, line)
}

/// Open a build phase (parsing, checking, compiling): a span that starts with [`Tukio::Hatua`].
#[inline]
pub fn phase(name: &str) -> Span {
    open(Tukio::Hatua, name, 0)
}

fn open(tukio: Tukio, name: &str, line: u32) -> Span {
    if !on() {
        return Span::none();
    }
    let depth = DEPTH.with(|d| {
        let depth = d.get();
        d.set(depth.saturating_add(1));
        depth
    });
    record(tukio, depth, name, line);
    Span {
        name: Some(name.to_string()),
    }
}

/// An open span (see [`enter`]).
#[must_use = "a span ends when this guard is dropped"]
pub struct Span {
    name: Option<String>,
}

impl Span {
    /// A guard that records nothing (where a span is optional).
    pub fn none() -> Span {
        Span { name: None }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(name) = self.name.take() {
            let depth = DEPTH.with(|d| {
                let depth = d.get().saturating_sub(1);
                d.set(depth);
                depth
            });
            if on() {
                record(Tukio::Toka, depth, &name, 0);
            }
        }
    }
}

/// Install the sink a specification names (`ASILI_FUATILIA`, `--fuatilia=...`):
/// `mti` or `json` (to standard error), `mti:<faili>`, `json:<faili>`, `binari:<faili>`.
pub fn install_spec(spec: &str) -> Result<(), String> {
    let (kind, path) = match spec.split_once(':') {
        Some((k, p)) => (k, Some(p)),
        None => (spec, None),
    };
    let out: Box<dyn Write + Send> = match path {
        Some(p) => Box::new(std::io::BufWriter::new(std::fs::File::create(p).map_err(
            |e| format!("imeshindwa kufungua faili ya ufuatiliaji {p}: {e}"),
        )?)),
        None => Box::new(std::io::BufWriter::new(std::io::stderr())),
    };
    let sink: Box<dyn Sink> = match kind {
        "" | "mti" => {
            let color = path.is_none()
                && std::io::stderr().is_terminal()
                && std::env::var_os("NO_COLOR").is_none();
            Box::new(TreeSink::new(out, color))
        }
        "json" => Box::new(JsonSink::new(out)),
        "binari" if path.is_some() => Box::new(BinarySink::new(out)),
        "binari" => return Err("binari inahitaji faili: tumia binari:<faili>".to_string()),
        _ => {
            return Err(format!(
                "namna ya ufuatiliaji '{kind}' haijulikani (tumia mti, json au binari:<faili>)"
            ))
        }
    };
    install(sink);
    Ok(())
}

/// Install the sink `ASILI_FUATILIA` names, if it is set. `Ok(true)` when tracing is now on.
pub fn install_from_env() -> Result<bool, String> {
    match std::env::var("ASILI_FUATILIA") {
        Ok(spec) if !spec.is_empty() => install_spec(&spec).map(|()| true),
        _ => Ok(false),
    }
}

/// The readable tree: one line per event, indented by depth, Swahili labels.
pub struct TreeSink {
    out: Box<dyn Write + Send>,
    color: bool,
    /// When each open span started, per thread (to show how long it took).
    started: HashMap<u64, Vec<u64>>,
}

impl TreeSink {
    pub fn new(out: Box<dyn Write + Send>, color: bool) -> Self {
        TreeSink {
            out,
            color,
            started: HashMap::new(),
        }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

fn line_note(line: u32) -> String {
    if line == 0 {
        String::new()
    } else {
        format!("  (mstari {line})")
    }
}

impl Sink for TreeSink {
    fn record(&mut self, e: &Event<'_>) {
        let indent = "│ ".repeat(e.depth as usize);
        let thread = if e.thread > 1 {
            format!("[uzi {}] ", e.thread)
        } else {
            String::new()
        };
        let text = match e.tukio {
            Tukio::Ingia => {
                self.started.entry(e.thread).or_default().push(e.time_ns);
                format!("▶ {}{}", self.paint("36", e.name), line_note(e.line))
            }
            Tukio::Toka => {
                let start = self.started.get_mut(&e.thread).and_then(Vec::pop);
                let took = start.map_or(String::new(), |s| {
                    format!("  {:.3} ms", (e.time_ns - s) as f64 / 1e6)
                });
                format!("◀ {}{}", e.name, took)
            }
            Tukio::Kosa => format!(
                "✖ {}: {}{}",
                self.paint("31", e.tukio.maelezo()),
                e.name,
                line_note(e.line)
            ),
            Tukio::Hatua => {
                self.started.entry(e.thread).or_default().push(e.time_ns);
                format!("◆ {}", self.paint("1", e.name))
            }
            other => format!("• {}: {}{}", other.maelezo(), e.name, line_note(e.line)),
        };
        let _ = writeln!(self.out, "{thread}{indent}{text}");
    }

    fn flush(&mut self) {
        let _ = self.out.flush();
    }
}

/// JSON lines in OpenTelemetry's shape: a span is written when it ends (with its start and end
/// times and its parent), every other event as a log record inside its span.
pub struct JsonSink {
    out: Box<dyn Write + Send>,
    /// Open spans per thread: (span id, start time, name, line).
    open: HashMap<u64, Vec<(u64, u64, String, u32)>>,
    next_span: u64,
    epoch_ns: u64,
    trace_id: String,
}

impl JsonSink {
    pub fn new(out: Box<dyn Write + Send>) -> Self {
        let epoch_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        JsonSink {
            out,
            open: HashMap::new(),
            next_span: 1,
            epoch_ns,
            trace_id: format!("{:016x}{:016x}", epoch_ns, std::process::id()),
        }
    }
}

/// `s` as a JSON string literal.
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Sink for JsonSink {
    fn record(&mut self, e: &Event<'_>) {
        let now = self.epoch_ns + e.time_ns;
        let stack = self.open.entry(e.thread).or_default();
        let line = match e.tukio {
            Tukio::Ingia | Tukio::Hatua => {
                stack.push((self.next_span, now, e.name.to_string(), e.line));
                self.next_span += 1;
                return;
            }
            Tukio::Toka => {
                let Some((id, start, name, src_line)) = stack.pop() else {
                    return;
                };
                let parent = stack.last().map_or(String::new(), |p| {
                    format!("\"parentSpanId\":\"{:016x}\",", p.0)
                });
                format!(
                    "{{\"kind\":\"span\",\"traceId\":\"{}\",\"spanId\":\"{id:016x}\",{parent}\"name\":{},\"startTimeUnixNano\":{start},\"endTimeUnixNano\":{now},\"attributes\":{{\"asili.mstari\":{src_line},\"asili.uzi\":{},\"asili.kina\":{}}}}}",
                    self.trace_id,
                    json_str(&name),
                    e.thread,
                    e.depth
                )
            }
            other => {
                let span = stack
                    .last()
                    .map_or(String::new(), |p| format!("\"spanId\":\"{:016x}\",", p.0));
                format!(
                    "{{\"kind\":\"event\",\"traceId\":\"{}\",{span}\"name\":\"{}\",\"timeUnixNano\":{now},\"body\":{},\"attributes\":{{\"asili.tukio\":{},\"asili.mstari\":{},\"asili.uzi\":{},\"asili.kina\":{}}}}}",
                    self.trace_id,
                    other.jina(),
                    json_str(e.name),
                    other as u8,
                    e.line,
                    e.thread,
                    e.depth
                )
            }
        };
        let _ = writeln!(self.out, "{line}");
    }

    fn flush(&mut self) {
        let _ = self.out.flush();
    }
}

/// Fixed 4-byte frames: event id, depth (saturating at 255), then the source line as a
/// big-endian 16-bit payload (saturating at 65535). Names are not recorded.
pub struct BinarySink {
    out: Box<dyn Write + Send>,
}

impl BinarySink {
    pub fn new(out: Box<dyn Write + Send>) -> Self {
        BinarySink { out }
    }
}

/// The 4-byte frame for an event.
pub fn frame(tukio: Tukio, depth: u16, payload: u32) -> [u8; 4] {
    let p = payload.min(0xFFFF) as u16;
    [tukio as u8, depth.min(0xFF) as u8, (p >> 8) as u8, p as u8]
}

impl Sink for BinarySink {
    fn record(&mut self, e: &Event<'_>) {
        let _ = self.out.write_all(&frame(e.tukio, e.depth, e.line));
    }

    fn flush(&mut self) {
        let _ = self.out.flush();
    }
}

/// A binary recording (see [`BinarySink`]) as the readable Swahili tree, one line per frame.
pub fn decode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut frames = bytes.chunks_exact(4);
    for f in frames.by_ref() {
        let indent = "  ".repeat(f[1] as usize);
        let payload = u16::from_be_bytes([f[2], f[3]]);
        let what = match Tukio::from_byte(f[0]) {
            Some(t) => format!("tukio: {}", t.maelezo()),
            None => format!("tukio lisilojulikana 0x{:02x}", f[0]),
        };
        let line = if payload == 0 {
            String::new()
        } else {
            format!(" — mstari {payload}")
        };
        out.push_str(&format!("{indent}└── {what}{line}\n"));
    }
    if !frames.remainder().is_empty() {
        out.push_str("fremu isiyokamilika mwishoni mwa faili\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex as StdMutex};

    /// A writer the test can read back.
    #[derive(Clone, Default)]
    struct Shared(Arc<StdMutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // One test drives the global sink so tests do not race over it.
    #[test]
    fn every_format_records_spans_and_events() {
        let tree = Shared::default();
        install(Box::new(TreeSink::new(Box::new(tree.clone()), false)));
        {
            let _kuu = enter("kuu", 1);
            emit(Tukio::Kigeuzi, "x", 2);
            let _f = enter("fib", 3);
            emit(Tukio::Kosa, "undani mno", 4);
        }
        finish();
        let text = String::from_utf8(tree.0.lock().unwrap().clone()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "▶ kuu  (mstari 1)");
        assert_eq!(lines[1], "│ • kutenga nafasi ya kigeuzi: x  (mstari 2)");
        assert_eq!(lines[2], "│ ▶ fib  (mstari 3)");
        assert_eq!(lines[3], "│ │ ✖ kosa: undani mno  (mstari 4)");
        assert!(lines[4].starts_with("│ ◀ fib"));
        assert!(lines[5].starts_with("◀ kuu"));

        let json = Shared::default();
        install(Box::new(JsonSink::new(Box::new(json.clone()))));
        {
            let _kuu = enter("kuu", 1);
            emit(Tukio::MwitoMfumo, "chapisha \"x\"", 2);
        }
        finish();
        let text = String::from_utf8(json.0.lock().unwrap().clone()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("{\"kind\":\"event\""), "{text}");
        assert!(
            lines[0].contains("\"body\":\"chapisha \\\"x\\\"\""),
            "{text}"
        );
        assert!(lines[1].starts_with("{\"kind\":\"span\""), "{text}");
        assert!(lines[1].contains("\"name\":\"kuu\""), "{text}");

        let bin = Shared::default();
        install(Box::new(BinarySink::new(Box::new(bin.clone()))));
        {
            let _kuu = enter("kuu", 0x1A40);
            emit(Tukio::Kigeuzi, "x", 7);
        }
        finish();
        let bytes = bin.0.lock().unwrap().clone();
        assert_eq!(&bytes[..4], &[0x01, 0x00, 0x1A, 0x40]);
        assert_eq!(&bytes[4..8], &[0x03, 0x01, 0x00, 0x07]);
        assert_eq!(&bytes[8..], &[0x02, 0x00, 0x00, 0x00]);
        assert_eq!(
            decode(&bytes),
            "└── tukio: kuingia — mstari 6720\n  └── tukio: kutenga nafasi ya kigeuzi — mstari 7\n└── tukio: kutoka\n"
        );

        // Off: nothing is recorded and spans cost nothing.
        assert!(!on());
        let _s = enter("kimya", 1);
        emit(Tukio::Kosa, "kimya", 1);
    }

    #[test]
    fn specifications_and_decoder_edges() {
        assert!(install_spec("binari").is_err());
        assert!(install_spec("rangi").is_err());
        assert_eq!(
            decode(&[0x7F, 0, 0, 0, 1]),
            "└── tukio lisilojulikana 0x7f\nfremu isiyokamilika mwishoni mwa faili\n"
        );
        assert_eq!(frame(Tukio::Kosa, 300, 70_000), [0x06, 0xFF, 0xFF, 0xFF]);
    }
}
