//! Data this crate defines rather than re-exports: the manifest table.
//!
//! It is the one thing in the facade that is not a path over somebody else's
//! item, because the table describes the surface the facade re-exports and has
//! no port of its own to live behind. The binary that prints it reads it from
//! here rather than keeping a second copy.

pub mod Manifest;
