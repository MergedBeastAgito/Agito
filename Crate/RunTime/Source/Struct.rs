//! Types: the port, its error, the runner that enforces it, the authority it
//! delegates to, the local stand-in for the reducer, and the call shapes every
//! other crate is written against.

pub mod Authority;
pub mod Call;
pub mod Error;
pub mod Port;
pub mod RunTime;
pub mod StandIn;
pub(crate) mod State;
