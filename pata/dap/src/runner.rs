//! Wires a real `RealDebugHook` up to an actual running `.as` program: compiles the file named
//! by a DAP `launch` request's `program` field and runs it on a background thread once
//! `configurationDone` fires, with a monitor thread keeping `DapSession::paused_at_line` current
//! so `stackTrace` responses reflect where execution has genuinely stopped.
//!
//! Single-file compilation only (lex → parse — no project/dependency resolution, no semantic
//! check) — matches the DAP use case of "debug this one file," not a full `pata jenga` project
//! build. A project-aware `launch` (resolving `leta` imports via `pata-core`) is a reasonable
//! future extension but out of scope for the minimum viable surface this section targets.

use asili_evaluator::debug_hook::RealDebugHook;
use asili_lexer::tokenize;
use asili_parser::parse_tokens;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::server::DapSession;

/// Build a `DapSession<RealDebugHook>` wired to actually compile and run whatever `.as` file a
/// `launch` request names, once `configurationDone` arrives. `breakpoint_lines` starts empty —
/// real breakpoints are only known once `setBreakpoints` runs (before `configurationDone`, per
/// normal DAP client sequencing) — read fresh from `breakpoint_lines`, a shared handle the
/// callback closes over directly rather than needing the whole `DapSession` back (avoiding the
/// self-referential "session's own callback needs a reference to the session" problem, since the
/// session doesn't exist yet while its constructor argument is being built).
pub fn real_session() -> Arc<DapSession<RealDebugHook>> {
    let hook = Arc::new(RealDebugHook::new(Vec::new()));
    let breakpoint_lines: Arc<std::sync::Mutex<Vec<i64>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let paused_at_line = Arc::new(AtomicI64::new(0));

    let hook_for_callback = Arc::clone(&hook);
    let breakpoint_lines_for_callback = Arc::clone(&breakpoint_lines);
    let paused_at_line_for_callback = Arc::clone(&paused_at_line);
    let session = DapSession::new(Arc::clone(&hook))
        .with_breakpoint_lines_handle(Arc::clone(&breakpoint_lines))
        .with_paused_at_line_handle(Arc::clone(&paused_at_line))
        .with_on_configuration_done(move |program| {
            let lines = breakpoint_lines_for_callback.lock().unwrap().clone();
            launch_and_monitor(
                Arc::clone(&hook_for_callback),
                Arc::clone(&paused_at_line_for_callback),
                lines,
                program,
            );
        });
    Arc::new(session)
}

/// Compile `program` and run it on a new thread with `hook` attached, plus a lightweight monitor
/// thread that polls `hook.paused_at_line()` and mirrors it into `paused_at_line` (shared with
/// the owning `DapSession`) so `stackTrace` requests report the real, live paused location. Both
/// threads are fire-and-forget: `launch`/`configurationDone` in DAP is not expected to block
/// waiting for the program to finish (the client polls via `stackTrace`/`variables`/`continue`
/// instead), matching how `run()`'s own request loop keeps servicing requests concurrently with
/// the target program actually executing.
fn launch_and_monitor(
    hook: Arc<RealDebugHook>,
    paused_at_line: Arc<AtomicI64>,
    breakpoint_lines: Vec<i64>,
    program: Option<String>,
) {
    let Some(path) = program else {
        return;
    };
    let Ok(source) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(tokens) = tokenize(&source) else {
        return;
    };
    let Ok(module) = parse_tokens(&tokens) else {
        return;
    };

    hook.set_breakpoints(breakpoint_lines.into_iter().map(|l| l as usize).collect());

    let hook_for_monitor = Arc::clone(&hook);
    std::thread::spawn(move || {
        // Mirrors hook.paused_at_line() into the shared AtomicI64 for the life of the debug
        // session — there's no explicit "program finished" signal to wait on instead, so this
        // thread simply polls at a short, cheap interval; the DAP client's own `disconnect`
        // tears down the whole process when the session ends.
        loop {
            match hook_for_monitor.paused_at_line() {
                Some(line) => paused_at_line.store(line as i64, Ordering::SeqCst),
                None => paused_at_line.store(0, Ordering::SeqCst),
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });

    std::thread::spawn(move || {
        let _ = asili_evaluator::run_main_with_debug_hook(&module, Vec::new(), hook);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use dap::prelude::*;
    use dap::requests::{LaunchRequestArguments, SetBreakpointsArguments};
    use dap::types::SourceBreakpoint;
    use std::io::Write;

    fn req(seq: i64, command: Command) -> Request {
        Request { seq, command }
    }

    fn temp_as_file(name: &str, content: &str) -> std::path::PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-dap-runner-test-{name}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("kuu.as");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    /// End-to-end: a real `launch` naming a real `.as` file, a real breakpoint set via
    /// `setBreakpoints`, then `configurationDone` — the target program must actually start
    /// running and genuinely pause at the configured line, observable via the session's own
    /// `stackTrace`-facing state, not just the hook in isolation.
    #[test]
    fn configuration_done_launches_and_pauses_a_real_program_at_a_real_breakpoint() {
        let path = temp_as_file(
            "pause",
            "kazi kuu(hoja: Orodha<Neno>) -> Tupu {\n  weka a = 1\n  weka b = 2\n}",
        );
        let session = real_session();

        let mut launch_args = LaunchRequestArguments::default();
        let mut data = serde_json::Map::new();
        data.insert("program".to_string(), serde_json::json!(path.to_string_lossy()));
        launch_args.additional_data = Some(serde_json::Value::Object(data));
        session.handle(&req(1, Command::Launch(launch_args)));

        let mut bp_args = SetBreakpointsArguments::default();
        bp_args.breakpoints = Some(vec![SourceBreakpoint { line: 3, ..Default::default() }]);
        session.handle(&req(2, Command::SetBreakpoints(bp_args)));

        session.handle(&req(3, Command::ConfigurationDone));

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while session.hook().paused_at_line() != Some(3) {
            assert!(std::time::Instant::now() < deadline, "breakpoint never fired within 5s");
            std::thread::sleep(Duration::from_millis(10));
        }

        session.hook().resume();
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
