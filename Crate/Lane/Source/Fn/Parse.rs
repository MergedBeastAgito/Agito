//! The port's view of a reply: read it, and report a refusal as one of the
//! manifest's seven codes.
//!
//! The reading itself lives in [`crate::Fn::Decode`], which has a variant per
//! way a reply can be wrong - a missing field, a share that is not a share, a
//! choice outside its own distribution. That vocabulary is deliberately finer
//! than `module.json`'s, because the two answer different questions: the
//! manifest tells a caller what to do next, and the decoder tells a person what
//! is wrong with the body in hand.
//!
//! This module is the single place the two are joined. [`Parse`] reads through
//! the decoder and keeps the decoder's own error inside `Error::Unparseable`'s
//! `cause`, so a caller that routes on the code loses
//! nothing it can still inspect: `error.cause()` narrows on the exact variant
//! without the message being parsed back out of a string.
//!
//! It stays a verb of its own rather than being folded into
//! [`crate::Fn::Ask::Ask`] because a captured body is still the thing the suite
//! tests: a caller holding a body and no request can call this and get the
//! crate's own error type back.

use crate::{
	Fn::Decode::Decode,
	Struct::{Answer::Answer, Error::Error},
};

/// Read one reply, as the port reports a failure.
pub fn Parse(Body:&str) -> Result<Answer, Error> { Decode(Body).map_err(Error::Decode) }
