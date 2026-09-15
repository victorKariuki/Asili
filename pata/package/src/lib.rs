//! Asili package manager.
//! Handles manifest parsing, dependency resolution, and lock file generation.

pub mod manifest;
pub mod lock;
pub mod resolver;
pub mod paths;
pub mod fetch;
pub mod constraints;
pub mod registry;

pub use manifest::{Dependency, DependencyTable};
pub use lock::{IntegrityMismatch, LockFile, LockedDependency};
pub use resolver::Resolver;
pub use paths::Paths;
pub use fetch::{fetch_git, hash_dir, FetchError};
pub use constraints::VersionConstraint;
pub use registry::{PackageMetadata, RegistryEntry, RegistrySource, LocalRegistry};
