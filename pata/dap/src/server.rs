//! `pata-dap`'s request/response handling: minimum viable DAP surface per
//! `docs/design/pata-implementation-spec.md` Section 20 — `initialize`, `launch`,
//! `setBreakpoints`, `configurationDone`, `continue`, `stackTrace`, `scopes`, `variables`,
//! `threads`, `disconnect`. Not the full DAP spec (no `stepIn`/`stepOut`/watch
//! expressions/conditional breakpoints in this pass) — enough to single-step through a running
//! `.as` program via breakpoints and inspect variables once `core/evaluator` implements
//! `DebugHook` for real.
//!
//! Structurally mirrors `pata-lsp/src/server.rs`'s shape (a `Backend`-equivalent struct holding
//! shared state, a request/response loop) even though DAP and LSP are different protocols with
//! different message shapes — same overall "hold state, dispatch on message type" pattern.
//!
//! Session state note: `DapSession` tracks breakpoint lines and the last-known paused line
//! itself (`breakpoint_lines`/`paused_at_line`), rather than through `DebugHook` — the contract's
//! own 3-method shape (`should_pause`/`resume`/`current_bindings`) has no "configure breakpoints"
//! or "what line are you at" method, since that's an intentionally minimal surface for whoever
//! implements it in `core/evaluator` later. A real hook implementation will need its own way to
//! learn which lines are breakpoints (e.g. a constructor parameter, not a trait method) — this
//! module's `breakpoint_lines` is `pata-dap`'s own bookkeeping for building correct
//! `SetBreakpoints`/`StackTrace` responses, independent of however a real hook tracks the same
//! information on the evaluator side.

use dap::prelude::*;
use dap::types::{Breakpoint, Capabilities, Scope, Source, StackFrame, Thread, Variable};
use std::io::{BufReader, BufWriter, Read, Write};
use std::sync::Arc;

use crate::hook::DebugHook;

pub struct DapSession<H: DebugHook> {
    hook: Arc<H>,
    breakpoint_lines: std::sync::Mutex<Vec<i64>>,
    source_path: std::sync::Mutex<Option<String>>,
    /// The line last reported as paused-at, for `stackTrace` responses. Updated by whatever
    /// caller observes a real pause (see `run`'s doc comment on how this connects to a real
    /// hook once one exists) — `0` before any pause has occurred.
    paused_at_line: std::sync::atomic::AtomicI64,
}

impl<H: DebugHook> DapSession<H> {
    pub fn new(hook: Arc<H>) -> Self {
        Self {
            hook,
            breakpoint_lines: std::sync::Mutex::new(Vec::new()),
            source_path: std::sync::Mutex::new(None),
            paused_at_line: std::sync::atomic::AtomicI64::new(0),
        }
    }

    /// Record that execution is now paused at `line` — called once a real hook's `should_pause`
    /// has actually blocked (see `run`), so a subsequent `stackTrace` request reports the real
    /// paused location rather than a stale or default one.
    pub fn set_paused_line(&self, line: i64) {
        self.paused_at_line.store(line, std::sync::atomic::Ordering::SeqCst);
    }

    /// Handle one request, returning the `Response` to send back plus any `Event`s that should
    /// follow it (DAP allows/expects events alongside responses — e.g. `initialize` is
    /// typically followed by an `initialized` event once the adapter is ready for
    /// `setBreakpoints`).
    pub fn handle(&self, req: &Request) -> (Response, Vec<Event>) {
        match &req.command {
            Command::Initialize(_) => {
                let capabilities = Capabilities {
                    supports_configuration_done_request: Some(true),
                    ..Default::default()
                };
                (
                    ok_response(req, ResponseBody::Initialize(capabilities)),
                    vec![Event::Initialized],
                )
            }
            Command::Launch(args) => {
                // `program` (the .as file to debug) arrives as an editor-specific launch.json
                // field, not a named LaunchRequestArguments field — DAP's own spec leaves this
                // to each adapter, carried in `additional_data`.
                if let Some(data) = &args.additional_data {
                    if let Some(program) = data.get("program").and_then(|v| v.as_str()) {
                        *self.source_path.lock().unwrap() = Some(program.to_string());
                    }
                }
                (ack(req), vec![])
            }
            Command::SetBreakpoints(args) => {
                let lines: Vec<i64> = args
                    .breakpoints
                    .as_ref()
                    .map(|bps| bps.iter().map(|b| b.line).collect())
                    .unwrap_or_default();
                *self.breakpoint_lines.lock().unwrap() = lines.clone();

                let source_path = self.source_path.lock().unwrap().clone();
                let breakpoints: Vec<Breakpoint> = lines
                    .iter()
                    .map(|&line| Breakpoint {
                        verified: true,
                        line: Some(line),
                        source: Some(Source {
                            path: source_path.clone(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .collect();
                (
                    ok_response(req, ResponseBody::SetBreakpoints(responses::SetBreakpointsResponse { breakpoints })),
                    vec![],
                )
            }
            Command::ConfigurationDone => (ack(req), vec![]),
            Command::Threads => {
                let threads = vec![Thread { id: 1, name: "kuu".to_string() }];
                (ok_response(req, ResponseBody::Threads(responses::ThreadsResponse { threads })), vec![])
            }
            Command::Continue(_) => {
                self.hook.resume();
                (
                    ok_response(req, ResponseBody::Continue(responses::ContinueResponse { all_threads_continued: Some(true) })),
                    vec![],
                )
            }
            Command::StackTrace(_) => {
                let source_path = self.source_path.lock().unwrap().clone();
                let line = self.paused_at_line.load(std::sync::atomic::Ordering::SeqCst);
                let stack_frames = vec![StackFrame {
                    id: 1,
                    name: "kuu".to_string(),
                    source: Some(Source { path: source_path, ..Default::default() }),
                    line,
                    column: 0,
                    ..Default::default()
                }];
                (
                    ok_response(req, ResponseBody::StackTrace(responses::StackTraceResponse {
                        stack_frames,
                        total_frames: Some(1),
                    })),
                    vec![],
                )
            }
            Command::Scopes(_) => {
                let scopes = vec![Scope {
                    name: "Locals".to_string(),
                    variables_reference: 1,
                    expensive: false,
                    ..Default::default()
                }];
                (ok_response(req, ResponseBody::Scopes(responses::ScopesResponse { scopes })), vec![])
            }
            Command::Variables(_) => {
                let variables: Vec<Variable> = self
                    .hook
                    .current_bindings()
                    .into_iter()
                    .map(|(name, value)| Variable {
                        name,
                        value,
                        variables_reference: 0,
                        ..Default::default()
                    })
                    .collect();
                (ok_response(req, ResponseBody::Variables(responses::VariablesResponse { variables })), vec![])
            }
            Command::Disconnect(_) => (ack(req), vec![Event::Terminated(None)]),
            other => (
                req.clone().error(&format!("amri haijatekelezwa: {}", command_name(other))),
                vec![],
            ),
        }
    }

    /// The breakpoint lines currently configured (last `setBreakpoints` call) — exposed so a
    /// real evaluator integration can read them when constructing its own `DebugHook`
    /// implementation (the trait itself has no "get breakpoints" method; this is `pata-dap`'s
    /// own bookkeeping, read by whatever wires a real hook to this session).
    pub fn breakpoint_lines(&self) -> Vec<i64> {
        self.breakpoint_lines.lock().unwrap().clone()
    }

    pub fn hook(&self) -> &Arc<H> {
        &self.hook
    }
}

/// Build a successful response carrying `body` — thin wrapper over the `dap` crate's own
/// `Request::success` (which consumes `self`), so call sites can pass `req: &Request` without
/// needing to clone at every call site themselves.
fn ok_response(req: &Request, body: ResponseBody) -> Response {
    req.clone().success(body)
}

/// An acknowledgement-only response — thin wrapper over the `dap` crate's own `Request::ack()`,
/// which picks the correct empty/marker `ResponseBody` per command automatically (e.g.
/// `ResponseBody::Attach`, `ResponseBody::ConfigurationDone`) rather than always `None`.
/// Falls back to a bodyless success response if `ack()` doesn't recognize the command (shouldn't
/// happen for the commands this module actually calls it on: `launch`, `configurationDone`,
/// `disconnect`).
fn ack(req: &Request) -> Response {
    req.clone().ack().unwrap_or_else(|_| Response {
        request_seq: req.seq,
        success: true,
        message: None,
        body: None,
        error: None,
    })
}

/// Bare command name for an unimplemented-command error message — debug-formatting the whole
/// `Command` (with its arguments) would be noisy there.
fn command_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::Attach(_) => "attach",
        Command::Next(_) => "next",
        Command::StepIn(_) => "stepIn",
        Command::StepOut(_) => "stepOut",
        Command::Pause(_) => "pause",
        Command::Evaluate(_) => "evaluate",
        Command::SetVariable(_) => "setVariable",
        Command::Restart(_) => "restart",
        _ => "haijulikani",
    }
}

/// Run the DAP server loop over `input`/`output` (real stdio in `main.rs`, or an in-memory
/// buffer pair in tests) until the input stream closes or a `disconnect` request is handled.
///
/// Does not itself drive `hook.should_pause` — that's the evaluator's job once `DebugHook` has
/// a real implementation (it calls `should_pause` from inside its own statement-execution loop,
/// not `pata-dap`). This loop only answers DAP protocol requests against `session`'s state.
pub fn run<H, R, W>(hook: Arc<H>, input: R, output: W)
where
    H: DebugHook,
    R: Read,
    W: Write,
{
    let mut server = dap::server::Server::new(BufReader::new(input), BufWriter::new(output));
    let session = DapSession::new(hook);

    loop {
        match server.poll_request() {
            Ok(Some(req)) => {
                let is_disconnect = matches!(req.command, Command::Disconnect(_));
                let (response, events) = session.handle(&req);
                if server.respond(response).is_err() {
                    break;
                }
                for event in events {
                    if server.send_event(event).is_err() {
                        break;
                    }
                }
                if is_disconnect {
                    break;
                }
            }
            Ok(None) => break, // input closed
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_hook::MockHook;
    use dap::requests::{
        ContinueArguments, InitializeArguments, LaunchRequestArguments, ScopesArguments,
        SetBreakpointsArguments, StackTraceArguments, VariablesArguments,
    };
    use dap::types::SourceBreakpoint;

    fn session() -> DapSession<MockHook> {
        DapSession::new(Arc::new(MockHook::new()))
    }

    fn req(seq: i64, command: Command) -> Request {
        Request { seq, command }
    }

    #[test]
    fn initialize_responds_with_capabilities_and_an_initialized_event() {
        let s = session();
        let (resp, events) = s.handle(&req(1, Command::Initialize(InitializeArguments::default())));
        assert!(resp.success);
        assert!(matches!(resp.body, Some(ResponseBody::Initialize(_))));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Initialized));
    }

    #[test]
    fn launch_acknowledges_and_stores_the_program_path() {
        let s = session();
        let args = LaunchRequestArguments {
            no_debug: None,
            restart_data: None,
            additional_data: Some(serde_json::json!({"program": "/tmp/mfano.as"})),
        };
        let (resp, _) = s.handle(&req(2, Command::Launch(args)));
        assert!(resp.success);

        // The stored path surfaces in a later stackTrace response's Source.
        let (st_resp, _) = s.handle(&req(3, Command::StackTrace(StackTraceArguments {
            thread_id: 1, start_frame: None, levels: None, format: None,
        })));
        let ResponseBody::StackTrace(body) = st_resp.body.unwrap() else { panic!("expected StackTrace body") };
        assert_eq!(body.stack_frames[0].source.as_ref().unwrap().path.as_deref(), Some("/tmp/mfano.as"));
    }

    #[test]
    fn set_breakpoints_returns_one_verified_breakpoint_per_line() {
        let s = session();
        let args = SetBreakpointsArguments {
            source: dap::types::Source::default(),
            breakpoints: Some(vec![
                SourceBreakpoint { line: 5, ..Default::default() },
                SourceBreakpoint { line: 9, ..Default::default() },
            ]),
            #[allow(deprecated)]
            lines: None,
            source_modified: None,
        };
        let (resp, _) = s.handle(&req(4, Command::SetBreakpoints(args)));
        assert!(resp.success);
        let ResponseBody::SetBreakpoints(body) = resp.body.unwrap() else { panic!("expected SetBreakpoints body") };
        assert_eq!(body.breakpoints.len(), 2);
        assert!(body.breakpoints.iter().all(|b| b.verified));
        assert_eq!(body.breakpoints[0].line, Some(5));
        assert_eq!(body.breakpoints[1].line, Some(9));

        assert_eq!(s.breakpoint_lines(), vec![5, 9]);
    }

    #[test]
    fn continue_resumes_the_hook() {
        let hook = Arc::new(MockHook::new());
        let s = DapSession::new(Arc::clone(&hook));

        // Pause the hook on a background thread, then continue via the session and confirm the
        // pause actually released -- a real behavioral check, not just "the call didn't error."
        hook.set_breakpoints(vec![3]);
        let hook_for_pause = Arc::clone(&hook);
        let pause_thread = std::thread::spawn(move || hook_for_pause.should_pause(3));

        for _ in 0..100 {
            if hook.did_pause() { break; }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(hook.did_pause());

        let (resp, _) = s.handle(&req(5, Command::Continue(ContinueArguments { thread_id: 1, single_thread: None })));
        assert!(resp.success);
        assert!(pause_thread.join().unwrap(), "continue must have released the paused thread");
    }

    #[test]
    fn stack_trace_reports_the_last_paused_line() {
        let s = session();
        s.set_paused_line(42);
        let (resp, _) = s.handle(&req(6, Command::StackTrace(StackTraceArguments {
            thread_id: 1, start_frame: None, levels: None, format: None,
        })));
        let ResponseBody::StackTrace(body) = resp.body.unwrap() else { panic!("expected StackTrace body") };
        assert_eq!(body.stack_frames[0].line, 42);
    }

    #[test]
    fn scopes_returns_a_locals_scope() {
        let s = session();
        let (resp, _) = s.handle(&req(7, Command::Scopes(ScopesArguments { frame_id: 1 })));
        let ResponseBody::Scopes(body) = resp.body.unwrap() else { panic!("expected Scopes body") };
        assert_eq!(body.scopes.len(), 1);
        assert_eq!(body.scopes[0].name, "Locals");
    }

    #[test]
    fn variables_reflects_the_hooks_real_bindings() {
        let hook = Arc::new(MockHook::new());
        hook.set_bindings(vec![("jumla".to_string(), "10".to_string())]);
        let s = DapSession::new(hook);

        let (resp, _) = s.handle(&req(8, Command::Variables(VariablesArguments {
            variables_reference: 1, filter: None, start: None, count: None, format: None,
        })));
        let ResponseBody::Variables(body) = resp.body.unwrap() else { panic!("expected Variables body") };
        assert_eq!(body.variables.len(), 1);
        assert_eq!(body.variables[0].name, "jumla");
        assert_eq!(body.variables[0].value, "10");
    }

    #[test]
    fn disconnect_acknowledges_and_sends_a_terminated_event() {
        let s = session();
        let (resp, events) = s.handle(&req(9, Command::Disconnect(Default::default())));
        assert!(resp.success);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Terminated(_)));
    }

    #[test]
    fn an_unimplemented_command_returns_a_real_error_response() {
        let s = session();
        let (resp, _) = s.handle(&req(10, Command::Pause(Default::default())));
        assert!(!resp.success, "an unimplemented command must fail, not silently succeed");
        assert!(matches!(resp.message, Some(responses::ResponseMessage::Error(_))));
    }

    /// Full end-to-end: real DAP wire-protocol bytes (`Content-Length: N\r\n\r\n<json>`) in,
    /// real wire-protocol bytes out — through the actual `run()` stdio loop, not just
    /// `DapSession::handle` in isolation. Matches the `dap` crate's own `server.rs` test pattern.
    #[test]
    fn run_handles_a_real_wire_protocol_request_end_to_end() {
        let request_json = r#"{"seq":1,"type":"request","command":"initialize","arguments":{"adapterID":"test"}}"#;
        let input = format!("Content-Length: {}\r\n\r\n{}", request_json.len(), request_json);

        let mut output = Vec::new();
        let hook = Arc::new(MockHook::new());
        run(hook, std::io::Cursor::new(input.into_bytes()), &mut output);

        let output_str = String::from_utf8(output).expect("valid utf8 output");
        assert!(output_str.contains("Content-Length:"), "output should be real wire-protocol framed, got: {output_str}");
        assert!(output_str.contains("\"success\":true"), "got: {output_str}");
    }
}
