//! `sawia`/`subiri`: tasks. Calling a `sawia kazi` starts a task and returns its `Ahadi`;
//! `subiri a` waits for it. While one task waits — for another task, a timer, a channel, a
//! socket — the others on the same thread run.
//!
//! Each task is a coroutine with its own stack (`corosensei`), so a task can wait anywhere, deep
//! inside native code and the builtins it calls, and native code needs nothing special. Each
//! thread has one single-threaded tokio runtime and `LocalSet`, made on first use, that drives
//! its tasks: a task runs until it must wait, hands the waiting (a future) to its driver, and is
//! resumed with the result. [`block_on`] is the one way anything waits: inside a task it
//! suspends the task; outside (the thread's own code) it runs every task until its future is
//! done.
//!
//! The host keeps per-context state that native code reads (call depth, the stack limit for
//! direct calls); each switch into and out of a task saves and restores it, and points
//! `crate::stack` at the task's stack. A program's top-level call finishes every task it
//! started before it returns ([`drain`]), so no suspended task outlives the host it runs on.
//! Cancelling a task (`ghairi`) is cooperative: its pending wait returns an error, which leaves
//! the task's code the ordinary way — a task's stack is never unwound or dropped mid-call.

use crate::value::{EvalError, Value};

/// The error a cancelled task's waits return.
#[cfg(not(target_arch = "wasm32"))]
fn cancelled() -> EvalError {
    EvalError::Panic("kazi imeghairiwa".into())
}

/// Host state native code reads that belongs to whichever code is running: saved and
/// restored at every switch.
pub(crate) trait Context {
    /// Swap the running code's state with `saved`.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn swap_context(&mut self, saved: &mut [u64; 2]);
    /// Call function `index` of the program.
    fn call_index(&mut self, index: usize, args: Vec<Value>) -> Result<Value, EvalError>;
}

#[cfg(target_arch = "wasm32")]
pub(crate) use eager::*;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;

/// A task as an `Ahadi` holds it.
pub struct Task {
    state: std::cell::RefCell<Option<Result<Value, EvalError>>>,
    cancel: std::cell::Cell<bool>,
    #[cfg(not(target_arch = "wasm32"))]
    done: tokio::sync::Notify,
    #[cfg(not(target_arch = "wasm32"))]
    cancel_notify: tokio::sync::Notify,
}

impl Task {
    fn new() -> Self {
        Task {
            state: std::cell::RefCell::new(None),
            cancel: std::cell::Cell::new(false),
            #[cfg(not(target_arch = "wasm32"))]
            done: tokio::sync::Notify::new(),
            #[cfg(not(target_arch = "wasm32"))]
            cancel_notify: tokio::sync::Notify::new(),
        }
    }

    /// The task's outcome, when it has finished.
    pub(crate) fn result(&self) -> Option<Result<Value, EvalError>> {
        self.state.borrow().clone()
    }

    pub(crate) fn is_done(&self) -> bool {
        self.state.borrow().is_some()
    }

    fn finish(&self, outcome: Result<Value, EvalError>) {
        *self.state.borrow_mut() = Some(outcome);
        #[cfg(not(target_arch = "wasm32"))]
        self.done.notify_waiters();
    }

    /// Ask the task to stop: its current (or next) wait returns [`cancelled`].
    pub(crate) fn cancel(&self) {
        if !self.is_done() {
            self.cancel.set(true);
            #[cfg(not(target_arch = "wasm32"))]
            self.cancel_notify.notify_waiters();
        }
    }
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            if self.is_done() {
                "imekwisha"
            } else {
                "inaendelea"
            }
        )
    }
}

/// Wait for `task`: its value, or the error it ended with.
pub(crate) fn subiri(task: &std::rc::Rc<Task>) -> Result<Value, EvalError> {
    loop {
        if let Some(outcome) = task.result() {
            return outcome;
        }
        wait_done(task)?;
    }
}

/// Wait for the first of `tasks` to finish: its index and outcome.
pub(crate) fn subiri_yoyote(
    tasks: &[std::rc::Rc<Task>],
) -> Result<(usize, Result<Value, EvalError>), EvalError> {
    loop {
        if let Some((i, t)) = tasks.iter().enumerate().find(|(_, t)| t.is_done()) {
            return Ok((i, t.result().unwrap_or(Ok(Value::Tupu))));
        }
        wait_any(tasks)?;
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{Context, Task};
    use crate::value::{EvalError, Value};
    use corosensei::stack::{DefaultStack, Stack};
    use corosensei::{Coroutine, CoroutineResult, Yielder};
    use std::any::Any;
    use std::cell::Cell;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;

    /// Each task's stack: committed only as it is used.
    const TASK_STACK: usize = 4 << 20;

    type AnyBox = Box<dyn Any>;
    type Waiting = Pin<Box<dyn Future<Output = AnyBox>>>;

    /// What a task is resumed with.
    enum Resume {
        Go(AnyBox),
        Cancelled,
    }

    /// What a task hands its driver: a future to wait for.
    struct Wait(Waiting);

    struct Runtime {
        rt: tokio::runtime::Runtime,
        local: tokio::task::LocalSet,
        live: Cell<usize>,
        idle: tokio::sync::Notify,
    }

    thread_local! {
        static RUNTIME: Rc<Runtime> = Rc::new(Runtime {
            rt: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap_or_else(|e| panic!("tokio: {e}")),
            local: tokio::task::LocalSet::new(),
            live: Cell::new(0),
            idle: tokio::sync::Notify::new(),
        });
        /// The running task's yielder (null: the thread's own code is running).
        static CURRENT: Cell<*const Yielder<Resume, Wait>> = const { Cell::new(std::ptr::null()) };
    }

    fn runtime() -> Rc<Runtime> {
        RUNTIME.with(Rc::clone)
    }

    /// Whether code is running inside a task.
    pub(crate) fn in_task() -> bool {
        !CURRENT.with(Cell::get).is_null()
    }

    /// Whether a wait on this thread must let tasks run (code is inside a task, or tasks are
    /// pending): then it waits through [`block_on`] instead of blocking the thread.
    pub(crate) fn tasks_active() -> bool {
        in_task() || RUNTIME.with(|rt| rt.live.get() > 0)
    }

    /// Wait until `ready()` holds, checking every millisecond — for what has no future to wait
    /// on (a thread finishing, a lock another thread holds) — while this thread's tasks run.
    pub(crate) fn wait_until(mut ready: impl FnMut() -> bool) -> Result<(), EvalError> {
        while !ready() {
            block_on(async { tokio::time::sleep(std::time::Duration::from_millis(1)).await })?;
        }
        Ok(())
    }

    /// Wait for `fut`: inside a task, suspend the task until it is done; otherwise run every task
    /// of this thread until it is done. `Err` when the waiting task was cancelled.
    pub(crate) fn block_on<F>(fut: F) -> Result<F::Output, EvalError>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let yielder = CURRENT.with(Cell::get);
        if yielder.is_null() {
            let rt = runtime();
            return Ok(rt.local.block_on(&rt.rt, fut));
        }
        // SAFETY: CURRENT points at the running task's yielder, on that task's own stack, which
        // lives as long as the task does — and the task is running (this is its code).
        let yielder = unsafe { &*yielder };
        let waiting: Waiting = Box::pin(async move { Box::new(fut.await) as AnyBox });
        match yielder.suspend(Wait(waiting)) {
            Resume::Go(out) => out
                .downcast::<F::Output>()
                .map(|b| *b)
                .map_err(|_| EvalError::Unknown("kazi_sawia: aina ya matokeo".into())),
            Resume::Cancelled => Err(super::cancelled()),
        }
    }

    /// Start function `index` as a task on `host` (which runs this thread's program and outlives
    /// every task: the top-level call drains them).
    pub(crate) fn spawn(host: *mut dyn Context, index: usize, args: Vec<Value>) -> Rc<Task> {
        let task = Rc::new(Task::new());
        let stack = match DefaultStack::new(TASK_STACK) {
            Ok(s) => s,
            Err(e) => {
                task.finish(Err(EvalError::Unknown(format!("kazi_sawia: stack: {e}"))));
                return task;
            }
        };
        // The stack's lowest usable address, with room for its guard page and a margin.
        let limit = stack.limit().get() + 64 * 1024;
        let yielder_cell: Rc<Cell<*const Yielder<Resume, Wait>>> =
            Rc::new(Cell::new(std::ptr::null()));
        let yc = yielder_cell.clone();
        let mut co =
            Coroutine::with_stack(stack, move |y: &Yielder<Resume, Wait>, first: Resume| {
                yc.set(y as *const _);
                CURRENT.with(|c| c.set(y as *const _));
                if let Resume::Cancelled = first {
                    return Err(super::cancelled());
                }
                // SAFETY: the host outlives every task on its thread (`drain` runs before its
                // top-level call returns), and only the running code uses it at a time.
                unsafe { (*host).call_index(index, args) }
            });
        let rt = runtime();
        rt.live.set(rt.live.get() + 1);
        let t = task.clone();
        let rt2 = rt.clone();
        rt.local.spawn_local(async move {
            let mut ctx = [0u64, u64::MAX];
            let mut input = if t.cancel.get() {
                Resume::Cancelled
            } else {
                Resume::Go(Box::new(()))
            };
            loop {
                // Switch in: this task's yielder, stack limit and host state.
                let outer_current = CURRENT.with(|c| c.replace(yielder_cell.get()));
                let outer_limit = crate::stack::set_limit(Some(limit));
                // SAFETY: as above.
                unsafe { (*host).swap_context(&mut ctx) };
                let step = co.resume(input);
                unsafe { (*host).swap_context(&mut ctx) };
                crate::stack::set_limit(outer_limit);
                CURRENT.with(|c| c.set(outer_current));
                match step {
                    CoroutineResult::Yield(Wait(fut)) => {
                        let cancelled = async {
                            loop {
                                let n = t.cancel_notify.notified();
                                if t.cancel.get() {
                                    return;
                                }
                                n.await;
                            }
                        };
                        input = tokio::select! {
                            out = fut => Resume::Go(out),
                            _ = cancelled => Resume::Cancelled,
                        };
                    }
                    CoroutineResult::Return(outcome) => {
                        t.finish(outcome);
                        break;
                    }
                }
            }
            rt2.live.set(rt2.live.get() - 1);
            if rt2.live.get() == 0 {
                rt2.idle.notify_waiters();
            }
        });
        task
    }

    /// Run until every task of this thread has finished (a top-level call does this before it
    /// returns). A no-op inside a task.
    pub(crate) fn drain() {
        if in_task() {
            return;
        }
        let rt = runtime();
        if rt.live.get() == 0 {
            return;
        }
        let rt2 = rt.clone();
        rt.local.block_on(&rt.rt, async move {
            while rt2.live.get() > 0 {
                let n = rt2.idle.notified();
                if rt2.live.get() == 0 {
                    break;
                }
                n.await;
            }
        });
    }

    pub(super) fn wait_done(task: &Rc<Task>) -> Result<(), EvalError> {
        let t = task.clone();
        block_on(async move {
            let n = t.done.notified();
            if !t.is_done() {
                n.await;
            }
        })
    }

    pub(super) fn wait_any(tasks: &[Rc<Task>]) -> Result<(), EvalError> {
        let ts: Vec<Rc<Task>> = tasks.to_vec();
        block_on(async move {
            let waits: Vec<Pin<Box<dyn Future<Output = ()>>>> = ts
                .iter()
                .map(|t| -> Pin<Box<dyn Future<Output = ()>>> {
                    let t = t.clone();
                    Box::pin(async move {
                        let n = t.done.notified();
                        if !t.is_done() {
                            n.await;
                        }
                    })
                })
                .collect();
            if !waits.is_empty() {
                select_first(waits).await;
            }
        })
    }

    /// The first of `futs` to finish.
    async fn select_first(mut futs: Vec<Pin<Box<dyn Future<Output = ()>>>>) {
        std::future::poll_fn(move |cx| {
            for f in futs.iter_mut() {
                if f.as_mut().poll(cx).is_ready() {
                    return std::task::Poll::Ready(());
                }
            }
            std::task::Poll::Pending
        })
        .await
    }

    /// Sleep `secs` seconds: only this task waits (outside a task, the others run meanwhile).
    pub(crate) fn lala(secs: f64) -> Result<(), EvalError> {
        let d = std::time::Duration::from_secs_f64(secs.max(0.0));
        // Built on first poll, inside the runtime (the timer needs its context).
        block_on(async move { tokio::time::sleep(d).await })
    }

    /// Wait at most `secs` for `task`: `None` when time ran out first.
    pub(crate) fn subiri_kwa_muda(
        task: &Rc<Task>,
        secs: f64,
    ) -> Result<Option<Result<Value, EvalError>>, EvalError> {
        let t = task.clone();
        let d = std::time::Duration::from_secs_f64(secs.max(0.0));
        let finished = block_on(async move {
            tokio::time::timeout(d, async move {
                let n = t.done.notified();
                if !t.is_done() {
                    n.await;
                }
            })
            .await
            .is_ok()
        })?;
        Ok(finished.then(|| task.result()).flatten())
    }
}

/// The browser build has no stacks to switch: a task runs to completion when it starts.
#[cfg(target_arch = "wasm32")]
mod eager {
    use super::{Context, Task};
    use crate::value::{EvalError, Value};
    use std::rc::Rc;

    pub(crate) fn tasks_active() -> bool {
        false
    }

    pub(crate) fn wait_until(mut ready: impl FnMut() -> bool) -> Result<(), EvalError> {
        while !ready() {
            std::hint::spin_loop();
        }
        Ok(())
    }

    /// Without a scheduler a future can only be run when it is already complete.
    pub(crate) fn block_on<F>(fut: F) -> Result<F::Output, EvalError>
    where
        F: std::future::Future + 'static,
        F::Output: 'static,
    {
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        let mut fut = std::pin::pin!(fut);
        match fut.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(v) => Ok(v),
            std::task::Poll::Pending => Err(EvalError::Unknown(
                "kusubiri hakupatikani kwenye kivinjari".into(),
            )),
        }
    }

    pub(crate) fn spawn(host: *mut dyn Context, index: usize, args: Vec<Value>) -> Rc<Task> {
        let task = Rc::new(Task::new());
        // SAFETY: called from the host's own call, with the host itself.
        task.finish(unsafe { (*host).call_index(index, args) });
        task
    }

    pub(crate) fn drain() {}

    pub(super) fn wait_done(_task: &Rc<Task>) -> Result<(), EvalError> {
        Ok(())
    }

    pub(super) fn wait_any(_tasks: &[Rc<Task>]) -> Result<(), EvalError> {
        Ok(())
    }

    pub(crate) fn lala(_secs: f64) -> Result<(), EvalError> {
        Ok(())
    }

    pub(crate) fn subiri_kwa_muda(
        task: &Rc<Task>,
        _secs: f64,
    ) -> Result<Option<Result<Value, EvalError>>, EvalError> {
        Ok(task.result())
    }
}
