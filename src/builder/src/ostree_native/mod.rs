//! native OSTree bare-user repo reading.
//!
//! implements read-only operations for OSTree repositories without requiring
//! the ostree CLI or libostree. supports bare-user mode repos only.

pub mod checkout;
pub mod commit;
pub mod dirtree;
pub mod objects;
pub mod refs;
pub mod repo;

pub use repo::OstreeRepo;
