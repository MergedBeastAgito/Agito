//! What this crate can do, one module per verb.

pub mod Ask;
pub mod Compose;
pub mod Load;
pub mod Model;
pub mod Parse;
pub mod Select;

pub use crate::Fn::{
	Ask::Ask,
	Compose::Compose,
	Load::{Load, Read},
	Model::{Bare, Price, Route},
	Parse::Parse,
	Select::{Choose, Select},
};
