//! A poster that answers from a canned body, with no socket in sight.
//!
//! The port is the crate's only way out, so a test that supplies this fake
//! exercises the whole round trip - select, compose, post, parse, meter - and
//! still reaches nothing. What it *records* is the other half of the
//! instrument: a call refused before it was posted, or posted with a body that
//! is not the lane's, leaves no trace in an answer and is obvious in the
//! record.
//!
//! It lives beside the tests rather than in `Source/`, for the reason the
//! catalog's fake gives: `Source/` is what ships, and a capability that shipped
//! a poster would be a capability that could reach the network. This is the
//! instrument, not the model.
//!
//! # It ignores the call's deadline, on purpose
//!
//! A real poster owns the in-flight bound. A fake that enforced it as well
//! would mask the crate's own check: [`AgenticLane::Ask`] refuses to post a
//! call the caller has already cancelled, and a test of that proves nothing if
//! the poster would have refused too. So the fake always answers, and
//! `a_cancelled_call_is_never_posted` reads an empty record rather than a
//! failure it cannot attribute.
//!
//! # Using it from another test binary
//!
//! Include it the way `Test/Lane.rs` does - `mod Fake;` beside the test root -
//! then name what it offers:
//!
//! ```ignore
//! mod Fake;
//!
//! use Fake::{Captured, Fake as Scripted};
//! ```
//!
//! The module and the struct cannot share a name in one scope, which is why the
//! import renames it. Nothing here is `pub(crate)`: a test binary is its own
//! crate, so the items a test reaches for have to be `pub`.

#![allow(dead_code)]

use std::{
	future,
	sync::{Arc, Mutex},
};

use AgenticLane::{Failure, Poster, Request};
use AgenticRuntime::{Call, FutureBox};

/// A reply the endpoint actually answered, kept as the bytes a poster returns.
///
/// Captured rather than composed, because the parser's job is to read what the
/// endpoint writes - a body this crate built for itself would agree with its own
/// assumptions and prove nothing. Two questions, one of each shape: a choice
/// with a distribution over both its options, and a score with no legend of its
/// own (the request supplies that, which is the other thing a canned body
/// cannot fake for itself).
pub const Captured:&str = r#"{
	"success": true,
	"result": {
		"model": "clef-flash",
		"answers": {
			"course": {
				"type": "choice",
				"choice": "closer",
				"probabilities": { "closer": 0.62, "farther": 0.38 }
			},
			"disagreement": { "type": "score", "score": 2 }
		},
		"usage": { "input_tokens": 375, "output_tokens": 0 }
	}
}"#;

/// A poster that answers what a test scripted, and records what it was asked.
///
/// Construct it with [`Fake::Answering`] or [`Fake::Failing`]; both hand back an
/// `Arc<Self>` so a test can keep a handle to read [`Fake::Posted`] after the
/// call while the decider holds the trait object.
pub struct Fake {
	/// The reply to return, or the failure to report.
	Reply:Result<String, Failure>,
	/// Every request it was handed, as the path and the body it would send.
	Asked:Mutex<Vec<(String, String)>>,
}

impl Fake {
	/// A poster that answers with one body, whatever it is asked.
	pub fn Answering(Body:impl Into<String>) -> Arc<Self> {
		Arc::new(Self { Reply:Ok(Body.into()), Asked:Mutex::new(Vec::new()) })
	}

	/// A poster that reports one failure instead of answering.
	pub fn Failing(Message:impl Into<String>, Detail:impl Into<String>) -> Arc<Self> {
		Arc::new(Self { Reply:Err(Failure::Of(Message).With(Detail)), Asked:Mutex::new(Vec::new()) })
	}

	/// Every request posted so far, in the order it was posted: the path it
	/// would have gone to and the body it would have sent.
	pub fn Posted(&self) -> Vec<(String, String)> { self.Asked.lock().unwrap().clone() }

	/// The one request posted, for a test that expects exactly one.
	///
	/// A test that reads this and finds two has a round trip that posted twice,
	/// which is the failure it was looking for - so the count is asserted here
	/// rather than left to each caller.
	pub fn Only(&self) -> (String, String) {
		let Posted = self.Posted();
		assert_eq!(Posted.len(), 1, "expected exactly one post, saw {}", Posted.len());
		Posted.into_iter().next().expect("one post")
	}
}

impl Poster for Fake {
	fn Post<'a>(&'a self, Request:&'a Request, _Call:Call) -> FutureBox<'a, Result<String, Failure>> {
		self.Asked.lock().unwrap().push((Request.Path(), Request.Text()));
		Box::pin(future::ready(self.Reply.clone()))
	}
}
