//! Outbound HTTP.
//!
//! Everything the server fetches on an agent's behalf goes through
//! [`egress`], which decides whether a request may leave at all and, when it
//! may, which route it takes.

pub mod egress;
