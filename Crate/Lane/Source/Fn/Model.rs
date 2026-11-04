//! Which model, which route, and what it costs.
//!
//! Three facts, and all three are the endpoint's rather than a caller's:
//!
//! - the route is `/ai/run/@cf/cloudflare/<name>`. The chat-completions route
//!   answers 400 to a `{model, state, questions}` body, because a decision model
//!   returns a probability per option rather than a message;
//! - the body names the bare model (`clef-flash`), while the path carries
//!   `@cf/cloudflare/` - so a caller that passed the full id to the body would
//!   be naming a model the endpoint does not have;
//! - the price is per million input tokens, measured on this account.
//!
//! [`Bare`] accepts either spelling, because a caller copying an id out of a
//! dashboard should not have to know which half of it belongs where.

use crate::Struct::Budget::ClefFlashPerMillionInput;

/// The route a decision model is asked on, up to the model name.
pub const Route:&str = "/ai/run/@cf/cloudflare/";

/// The prefix a full model id carries and the body must not.
pub const Prefix:&str = "@cf/cloudflare/";

/// Every model this crate knows the price of: the alias a caller may pass, the
/// bare name the body wants, and the price per million input tokens.
pub const Models:[(&str, &str, f64); 2] =
	[("clef-flash", "clef-flash", ClefFlashPerMillionInput), ("clef", "clef", 0.24)];

/// The bare model name the body and the path want.
///
/// Either spelling is accepted, and a name this crate has never heard of is
/// passed through rather than replaced: a new model is a name the endpoint
/// knows, and silently asking a different one would be the worst kind of
/// fallback. An empty name is the one refusal, because an empty path segment is
/// not a model.
pub fn Bare(Name:&str) -> String {
	let Trimmed = Name.trim().trim_start_matches(Prefix).trim();
	if Trimmed.is_empty() { Models[0].1.to_string() } else { Trimmed.to_string() }
}

/// What a model charges per million input tokens, when this crate knows it.
///
/// An unknown model answers the `clef-flash` price rather than zero: the line a
/// run ends with is a report, and a report that says a model was free is worse
/// than one that is out of date. A caller that knows better passes its own price
/// to [`crate::Struct::Budget::Budget::At`].
pub fn Price(Name:&str) -> f64 {
	let Bare = Bare(Name);
	Models
		.iter()
		.find(|(Alias, Known, _)| *Alias == Bare || *Known == Bare)
		.map(|(_, _, Price)| *Price)
		.unwrap_or(ClefFlashPerMillionInput)
}
