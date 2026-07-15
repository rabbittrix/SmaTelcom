//! Legacy audit module — re-exports Phase-4 `db` types for compatibility.

#![allow(unused_imports)]
pub use crate::db::{AuditLogEntry as AuditRecord, ImpactReport};
