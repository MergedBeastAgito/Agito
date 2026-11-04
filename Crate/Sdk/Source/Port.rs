//! The whole surface of each port, at one path.
//!
//! [`Browse`] is `agentic.browse`, [`Chat`] is `agentic.chat.ask`, [`Design`]
//! is `agentic.verify.design`, [`Skill`] is `agentic.skill.validate`,
//! [`Runtime`] is `agentic.runtime` and [`Adapter`] is `agentic.adapter`. Each
//! name is the short form of a port, not of a crate: a consumer is choosing a
//! capability, and the package that happens to implement it is the thing this
//! facade exists to hide.
//!
//! The crate root carries the items a consumer needs first - a browser session,
//! the call that bounds it, the adapter port, the manifest table. These modules
//! carry everything else, including the names that collide across ports: five
//! crates ship an `Error`, four ship a `Create`, three ship a `Target`, and only
//! one of each can be at the root.

pub mod Adapter;
pub mod Browse;
pub mod Chat;
pub mod Design;
pub mod Runtime;
pub mod Skill;
