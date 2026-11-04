//! The whole surface of each port, at one path.
//!
//! [`Port::Browse`] is `agentic.browse`, [`Port::Chat`] is `agentic.chat.ask`,
//! [`Port::Design`] is `agentic.verify.design`, [`Port::Skill`] is
//! `agentic.skill.validate` and [`Port::Runtime`] is `agentic.runtime`. Each
//! name is the short form of a port, not of a crate: a consumer is choosing a
//! capability, and the package that happens to implement it is the thing this
//! facade exists to hide.
//!
//! The crate root carries the items a consumer needs first - a browser session,
//! the call that bounds it, the manifest table. These modules carry everything
//! else, including the names that collide across ports: four crates ship an
//! `Error`, four ship a `Create`, three ship a `Target`, and only one of each can
//! be at the root.

pub mod Browse;
pub mod Chat;
pub mod Design;
pub mod Runtime;
pub mod Skill;
