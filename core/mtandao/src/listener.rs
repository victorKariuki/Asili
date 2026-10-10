//! Listening sockets. A [`Listener`] is shared between threads (`Arc`); each thread accepts
//! through its own [`LocalListener`], registered with that thread's event loop. Stopping a
//! listener ends every pending and later `accept`, on every thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use socket2::{Domain, Protocol, Socket, Type};
use tokio::sync::Notify;

use crate::stream::Stream;

/// How to listen.
#[derive(Clone, Debug, Default)]
pub struct BindOptions {
    /// Let several sockets (processes) listen on the same port (`SO_REUSEPORT`, Unix).
    pub reuse_port: bool,
    /// How many connections may wait to be accepted (default 1024).
    pub backlog: Option<i32>,
}

enum Inner {
    Tcp(std::net::TcpListener),
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixListener, std::path::PathBuf),
}

/// A listening socket.
pub struct Listener {
    inner: Inner,
    stopped: AtomicBool,
    stop: Notify,
}

/// Listen on `addr`.
pub fn bind(addr: &str, opts: &BindOptions) -> Result<Listener, String> {
    let inner = match addr.strip_prefix("unix:") {
        Some(path) => bind_unix(path)?,
        None => bind_tcp(addr, opts)?,
    };
    Ok(Listener {
        inner,
        stopped: AtomicBool::new(false),
        stop: Notify::new(),
    })
}

fn bind_tcp(addr: &str, opts: &BindOptions) -> Result<Inner, String> {
    use std::net::ToSocketAddrs;
    let addrs = addr
        .to_socket_addrs()
        .map_err(|e| format!("kusikiliza {addr}: {e}"))?;
    let mut last = format!("kusikiliza {addr}: hakuna anwani");
    for a in addrs {
        let attempt = || -> std::io::Result<std::net::TcpListener> {
            let sock = Socket::new(Domain::for_address(a), Type::STREAM, Some(Protocol::TCP))?;
            // Restarting a server must not wait out the old sockets' TIME_WAIT.
            #[cfg(unix)]
            sock.set_reuse_address(true)?;
            #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos"))))]
            if opts.reuse_port {
                sock.set_reuse_port(true)?;
            }
            sock.bind(&a.into())?;
            sock.listen(opts.backlog.unwrap_or(1024))?;
            sock.set_nonblocking(true)?;
            Ok(sock.into())
        };
        match attempt() {
            Ok(l) => return Ok(Inner::Tcp(l)),
            Err(e) => last = format!("kusikiliza {a}: {e}"),
        }
    }
    Err(last)
}

#[cfg(unix)]
fn bind_unix(path: &str) -> Result<Inner, String> {
    use std::os::unix::fs::FileTypeExt;
    // A socket file left by a server that is gone: nothing answers on it, so it can go.
    if std::fs::metadata(path).is_ok_and(|m| m.file_type().is_socket())
        && std::os::unix::net::UnixStream::connect(path).is_err()
    {
        let _ = std::fs::remove_file(path);
    }
    let l = std::os::unix::net::UnixListener::bind(path)
        .map_err(|e| format!("kusikiliza unix:{path}: {e}"))?;
    l.set_nonblocking(true)
        .map_err(|e| format!("kusikiliza unix:{path}: {e}"))?;
    Ok(Inner::Unix(l, path.into()))
}

#[cfg(not(unix))]
fn bind_unix(path: &str) -> Result<Inner, String> {
    Err(format!("soketi za unix hazipatikani hapa: {path}"))
}

impl Listener {
    /// The address it listens on (with the real port when bound to port 0).
    pub fn local_addr(&self) -> String {
        match &self.inner {
            Inner::Tcp(l) => l
                .local_addr()
                .map_or_else(|_| String::new(), |a| a.to_string()),
            #[cfg(unix)]
            Inner::Unix(_, p) => format!("unix:{}", p.display()),
        }
    }

    /// Stop: every pending and later `accept` returns `None`.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.stop.notify_waiters();
    }

    /// Whether [`Listener::stop`] was called.
    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    /// This listener, registered with the calling thread's event loop (call it inside one).
    pub fn local(self: &Arc<Self>) -> Result<LocalListener, String> {
        let err = |e: std::io::Error| format!("kusikiliza: {e}");
        let inner = match &self.inner {
            Inner::Tcp(l) => LocalInner::Tcp(
                tokio::net::TcpListener::from_std(l.try_clone().map_err(err)?).map_err(err)?,
            ),
            #[cfg(unix)]
            Inner::Unix(l, _) => LocalInner::Unix(
                tokio::net::UnixListener::from_std(l.try_clone().map_err(err)?).map_err(err)?,
            ),
        };
        Ok(LocalListener {
            shared: self.clone(),
            inner,
        })
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Inner::Unix(_, path) = &self.inner {
            let _ = std::fs::remove_file(path);
        }
    }
}

enum LocalInner {
    Tcp(tokio::net::TcpListener),
    #[cfg(unix)]
    Unix(tokio::net::UnixListener),
}

/// A [`Listener`] on one thread's event loop.
pub struct LocalListener {
    shared: Arc<Listener>,
    inner: LocalInner,
}

impl LocalListener {
    /// The listener this accepts for.
    pub fn shared(&self) -> &Arc<Listener> {
        &self.shared
    }

    /// The next connection and its peer's address; `None` once the listener is stopped.
    pub async fn accept(&self) -> Result<Option<(Stream, String)>, String> {
        let stopped = self.shared.stop.notified();
        tokio::pin!(stopped);
        stopped.as_mut().enable();
        if self.shared.is_stopped() {
            return Ok(None);
        }
        tokio::select! {
            biased;
            _ = &mut stopped => Ok(None),
            r = self.accept_one() => r.map(Some),
        }
    }

    async fn accept_one(&self) -> Result<(Stream, String), String> {
        let err = |e: std::io::Error| format!("kukubali: {e}");
        match &self.inner {
            LocalInner::Tcp(l) => {
                let (s, a) = l.accept().await.map_err(err)?;
                let _ = s.set_nodelay(true);
                Ok((Stream::Tcp(s), a.to_string()))
            }
            #[cfg(unix)]
            LocalInner::Unix(l) => {
                let (s, a) = l.accept().await.map_err(err)?;
                Ok((Stream::Unix(s), crate::stream::unix_addr(Some(a))))
            }
        }
    }
}
