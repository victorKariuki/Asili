//! Asili package manager.
//! Handles manifest parsing, dependency resolution, and lock file generation.

pub mod manifest;
pub mod lock;
pub mod resolver;
pub mod workspace;
pub mod paths;

pub use manifest::{Manifest, Dependency, WorkspaceConfig};
pub use lock::{LockFile, LockedDependency};
pub use resolver::Resolver;
pub use workspace::Workspace;
pub use paths::Paths;
