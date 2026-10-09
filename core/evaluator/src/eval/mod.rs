//! The language's value semantics, shared by everything that runs a program: operators
//! (`ops`), methods, indexing, casts and iteration (`methods`), and patterns (`pattern`).

pub(crate) mod methods;
pub(crate) mod ops;
pub(crate) mod pattern;
