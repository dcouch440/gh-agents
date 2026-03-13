//! Repository traits for database operations.
//!
//! Each trait abstracts the DB operations for a specific domain module.
//! Production code uses `PgRepo` (see `pg_repo/`). Tests use `MockXxxRepo` from mockall.

mod agent;
mod collection;
mod content_version;
mod document;
mod execution;
mod protocol;
mod room;
mod session;
mod system;
mod system_file;
mod workflow;

pub use agent::*;
pub use collection::*;
pub use content_version::*;
pub use document::*;
pub use execution::*;
pub use protocol::*;
pub use room::*;
pub use session::*;
pub use system::*;
pub use system_file::*;
pub use workflow::*;
