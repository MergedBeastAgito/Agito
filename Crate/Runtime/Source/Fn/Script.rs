//! Filling a script's placeholders.
//!
//! The expressions this workspace sends to a page are JavaScript files, not
//! string literals assembled by `format!`. A file cannot interpolate, so a
//! value the caller supplies is written where it belongs as a `{{name}}`
//! placeholder and replaced at call time. That is the whole of the mechanism,
//! and it is here rather than in either crate that uses it because both a
//! capability and the workflow that composes it need it, and two copies of a
//! substitution rule is two chances for the rule to differ.
//!
//! What the placeholder must not be is silent. A script whose caller forgets a
//! parameter keeps `{{busy}}` in the expression, and a page that receives it
//! throws a `SyntaxError` - a failure with a reason, not a selector that
//! quietly matches nothing. [`Unfilled`] is the same check without a page: it
//! names the placeholder a string still carries, so a test can assert that
//! every script is filled completely and that the check itself bites.

/// Replace every `{{name}}` in `Template` with the value `Parameters` gives it.
///
/// A parameter the template does not name is not an error - it is simply not
/// used - because a caller may build one parameter set for several scripts. A
/// placeholder no parameter names is left in place, deliberately: [`Unfilled`]
/// is what reports it.
pub fn Fill(Template:&str, Parameters:&[(&str, &str)]) -> String {
	let mut Text = Template.to_string();
	for (Name, Value) in Parameters {
		Text = Text.replace(&format!("{{{{{Name}}}}}"), Value);
	}
	Text
}

/// The name of the first placeholder `Text` still carries, if it carries one.
pub fn Unfilled(Text:&str) -> Option<String> {
	let Start = Text.find("{{")?;
	let Rest = &Text[Start + 2..];
	let End = Rest.find("}}")?;
	Some(Rest[..End].to_string())
}
