//! The language's value semantics, shared by everything that runs a program: operators
//! (`ops`), methods, indexing, casts and iteration (`methods`), and patterns (`pattern`).

// Runtime code never panics on its own: an impossible state is an error the program sees
// (and its safe state handles), not a crash (see docs/design/safety-critical-roadmap.md §3).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

pub(crate) mod methods;
pub(crate) mod ops;
pub(crate) mod pattern;
