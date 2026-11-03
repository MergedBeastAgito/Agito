//! Functions: the expressions sent to a page, and the readers that turn what
//! came back into the port's types.
//!
//! Nothing here decides anything. A reader answers `None` when the page
//! answered something that is not the shape the port promised, and the driver
//! turns that `None` into one of its four codes rather than into a panic: a
//! provider that answers a number where a label was asked for must not be able
//! to make `ask` fail outside the vocabulary the manifest declares.

pub mod Expressions;
pub mod Read;
