//! Asili package manager.
//! Handles manifest parsing, dependency resolution, and lock file generation.

pub mod constraints;
pub mod fetch;
pub mod lock;
pub mod manifest;
pub mod paths;
pub mod registry;
pub mod remote_registry;
pub mod resolver;

pub use constraints::VersionConstraint;
pub use fetch::{fetch_git, hash_dir, FetchError};
pub use lock::{IntegrityMismatch, LockFile, LockedDependency};
pub use manifest::{Dependency, DependencyTable};
pub use paths::Paths;
pub use registry::{LocalRegistry, PackageMetadata, RegistryEntry, RegistrySource};
pub use remote_registry::{
    fetch_and_verify, fetch_index, RemoteIndexEntry, DEFAULT_INDEX_BASE_URL,
};
pub use resolver::Resolver;
