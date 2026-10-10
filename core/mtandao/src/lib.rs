//! Asili's network layer: TCP, TLS and Unix-socket streams and listeners, UDP sockets and DNS,
//! on tokio.
//! Every network builtin (sockets, the HTTP client and server) connects, listens and handles
//! TLS here, once. The futures run on the calling thread's event loop
//! (`asili_evaluator::kazi_sawia`), so a `sawia` task waiting on the network lets the thread's
//! other tasks run.
//!
//! Addresses are `"mwenyeji:mlango"` (`"[::1]:80"` for IPv6) or `"unix:/njia/ya/soketi"`.
//! Errors are messages for the program (Swahili, with the OS's own text).

mod dns;
pub mod http;
mod listener;
mod stream;
pub mod tls;
mod udp;
pub mod ws;

pub use dns::resolve;
pub use listener::{bind, BindOptions, Listener, LocalListener};
pub use stream::{connect, split_host_port, ConnectOptions, Stream};
pub use udp::{udp_bind, Udp};
