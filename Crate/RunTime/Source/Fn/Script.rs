//! Filling a script's parameters.
//!
//! The expressions this workspace sends to a page are TypeScript files under
//! `Source/Script/`, compiled to JavaScript by a crate's `build.rs` and baked
//! into the binary by [`script!`](crate::script). A compiled expression still
//! carries its parameters: a value the caller supplies is written where it
//! belongs as a `__PARAM_Name__` marker, declared in the script by
//! `declare const __PARAM_Name__: T;`, and replaced at call time by
//! [`FillDeclared`].
//!
//! # Why a marker is an identifier
//!
//! The previous form was `{{name}}`, and it was not parseable: a file holding
//! one could not be compiled, so the expression was never seen by any tool
//! between being written and being sent. `__PARAM_Name__` is a valid
//! TypeScript identifier, so the file the compiler checks is the file the page
//! runs, and the only difference between them is the markers.
//!
//! # Why the kind is declared rather than guessed
//!
//! A parameter is one of two things, and confusing them is how a selector
//! breaks an expression. A parameter declared `string` is **data**: it is
//! substituted as an escaped JavaScript string literal, so a selector holding
//! `"`, `\` or a newline cannot end the string it sits in. Any other declared
//! type is **code**: it is spliced as it stands, because it is a function or a
//! regular expression the expression calls. The declaration is the single
//! statement of which it is - [`Escape`] is applied to one kind and never to
//! the other.
//!
//! # Why the failure is loud
//!
//! A marker no value replaces stays in the expression, and a page that receives
//! `__PARAM_Scroll__` throws `ReferenceError: __PARAM_Scroll__ is not defined`;
//! a failure with a name, not a selector that quietly matches nothing.
//! [`Unfilled`] is the same check without a page, and [`AssertParameters`]
//! moves the parameter check to compile time: a call site that forgets a
//! parameter, passes one the script does not declare, or passes the same one
//! twice, fails `cargo build` instead of a page.

use std::fmt::Write;

/// Fill a script from the parameter contract its build wrote.
///
/// `Declared` is the script's `.params` file, one `Name:kind` per line, in the
/// order the script declares them. A `text` parameter is escaped into a
/// JavaScript string literal; a `code` parameter is spliced as it stands.
///
/// One pass, left to right, over the template's own markers: a value is written
/// into the output and never looked at again. That matters because a value can
/// be a page's text, and a page whose question happens to contain
/// `__PARAM_Seen__` would otherwise have a later parameter's code spliced into
/// the middle of the string it sits in - substitution that can break the
/// expression is exactly what escaping exists to prevent.
pub fn FillDeclared(Template:&str, Declared:&str, Values:&[(&str, &str)]) -> String {
	let mut Out = String::with_capacity(Template.len());
	let mut Rest = Template;
	while let Some(At) = Rest.find("__PARAM_") {
		Out.push_str(&Rest[..At]);
		let After = &Rest[At + "__PARAM_".len()..];
		let Some(End) = After.find("__") else {
			// An unterminated marker is not a marker: the rest is text, and
			// `Unfilled` is what reports it.
			Out.push_str(&Rest[At..]);
			return Out;
		};
		let Name = &After[..End];
		match Substitution(Declared, Name, Values) {
			Some(Replacement) => Out.push_str(&Replacement),
			// No value names it: leave the marker, so the page says which
			// parameter was missing instead of the expression quietly changing.
			None => Out.push_str(&Rest[At..At + "__PARAM_".len() + End + 2]),
		}
		Rest = &After[End + 2..];
	}
	Out.push_str(Rest);
	Out
}

/// What `Name` becomes: escaped when the script declares it `text`, as it
/// stands when it declares anything else.
fn Substitution(Declared:&str, Name:&str, Values:&[(&str, &str)]) -> Option<String> {
	let Kind = Declared
		.lines()
		.find_map(|Line| Line.split_once(':').filter(|(Declared, _)| *Declared == Name))
		.map(|(_, Kind)| Kind.trim())?;
	let (_, Value) = Values.iter().find(|(Given, _)| *Given == Name)?;
	Some(match Kind {
		"text" => Escape(Value),
		_ => (*Value).to_string(),
	})
}

/// `Value` as a JavaScript string literal, quotes included.
///
/// The escaping is the one a JSON string already needs, which is the whole of
/// what a JavaScript double-quoted string needs: a backslash, a quote, the
/// control characters with a short form, and any other control character as
/// `\u00XX`. Everything else - including non-ASCII, and including the two line
/// separators `U+2028` and `U+2029` - is left as it is. Those two are the only
/// characters where JSON and a JavaScript literal used to disagree; since
/// ES2019 a string literal may contain them raw, and escaping them would change
/// the bytes of an expression a page already receives for a hazard no current
/// engine has. `escaping_matches_json_for_every_byte` holds this to
/// `serde_json`, which is the encoder these expressions were built with before
/// they were compiled.
pub fn Escape(Value:&str) -> String {
	let mut Out = String::with_capacity(Value.len() + 2);
	Out.push('"');
	for Character in Value.chars() {
		match Character {
			'"' => Out.push_str("\\\""),
			'\\' => Out.push_str("\\\\"),
			'\n' => Out.push_str("\\n"),
			'\r' => Out.push_str("\\r"),
			'\t' => Out.push_str(r"\t"),
			'\u{08}' => Out.push_str("\\b"),
			'\u{0c}' => Out.push_str("\\f"),
			Control if (Control as u32) < 0x20 => {
				let _ = write!(Out, "\\u{:04x}", Control as u32);
			},
			Other => Out.push(Other),
		}
	}
	Out.push('"');
	Out
}

/// The name of the first marker `Text` still carries, if it carries one.
pub fn Unfilled(Text:&str) -> Option<String> {
	let Start = Text.find("__PARAM_")?;
	let Rest = &Text[Start + "__PARAM_".len()..];
	let End = Rest.find("__")?;
	Some(Rest[..End].to_string())
}

/// Fail the build when a call site's parameters and the script's declarations
/// disagree.
///
/// Called by [`script!`](crate::script) in a `const` context, so a missing
/// parameter, an extra one, or a name the script does not declare is a compile
/// error at the call site rather than a marker that reaches a page. The count
/// is compared as well as the names, because `every given name is declared`
/// plus `the counts agree` is only a bijection when no name repeats - and a
/// repeated name is a mistake the count catches.
///
/// A `const fn` cannot format its panic message, so the message names the
/// problem rather than the parameter, and the call site's own line is in the
/// compiler's error.
pub const fn AssertParameters(Declared:&str, Given:&[&str]) {
	let mut Index = 0;
	while Index < Given.len() {
		if !Declares(Declared, Given[Index]) {
			panic!("script parameters: a parameter the script does not declare");
		}
		let mut Other = 0;
		while Other < Index {
			if Same(Given[Other], Given[Index]) {
				panic!("script parameters: the same parameter twice");
			}
			Other += 1;
		}
		Index += 1;
	}
	if CountDeclared(Declared) != Given.len() {
		panic!("script parameters: a declared parameter is missing");
	}
}

/// Whether two strings are equal, byte for byte, in a `const` context.
const fn Same(Left:&str, Right:&str) -> bool {
	let (Left, Right) = (Left.as_bytes(), Right.as_bytes());
	if Left.len() != Right.len() {
		return false;
	}
	let mut Index = 0;
	while Index < Left.len() {
		if Left[Index] != Right[Index] {
			return false;
		}
		Index += 1;
	}
	true
}

/// How many parameters `Declared` declares: one per non-empty line.
const fn CountDeclared(Declared:&str) -> usize {
	let Bytes = Declared.as_bytes();
	let mut Count = 0;
	let mut At = 0;
	let mut Line = 0;
	while At < Bytes.len() {
		if Bytes[At] == b'\n' {
			if Line > 0 {
				Count += 1;
			}
			Line = 0;
		} else {
			Line += 1;
		}
		At += 1;
	}
	if Line > 0 {
		Count += 1;
	}
	Count
}

/// Whether `Declared` names `Name`, as a whole line's name rather than a
/// substring of one.
const fn Declares(Declared:&str, Name:&str) -> bool {
	let Bytes = Declared.as_bytes();
	let Wanted = Name.as_bytes();
	let mut At = 0;
	while At < Bytes.len() {
		// The start of a line: the name runs to the `:` that ends it.
		let Start = At;
		let mut End = At;
		while End < Bytes.len() && Bytes[End] != b':' && Bytes[End] != b'\n' {
			End += 1;
		}
		if End - Start == Wanted.len() {
			let mut Same = true;
			let mut Index = 0;
			while Index < Wanted.len() {
				if Bytes[Start + Index] != Wanted[Index] {
					Same = false;
					break;
				}
				Index += 1;
			}
			if Same && End < Bytes.len() && Bytes[End] == b':' {
				return true;
			}
		}
		// Past this line.
		At = End;
		while At < Bytes.len() && Bytes[At] != b'\n' {
			At += 1;
		}
		At += 1;
	}
	false
}

/// Bake a compiled script into the binary, filled from its declared
/// parameters.
///
/// ```ignore
/// let Expression = script!("Expressions/ReadMode"; Mode = &Selectors.Mode);
/// ```
///
/// The path is the script's path under `Source/Script/`, without the
/// extension, and it is compiled by the calling crate's `build.rs` into that
/// crate's `OUT_DIR` - which is why `env!("OUT_DIR")` is inside the macro: it
/// expands where the macro is called, so each crate reads its own scripts.
///
/// A parameter list is checked at compile time against the script's own
/// declarations: a parameter the script does not declare, a declared one the
/// call site forgot, or the same one twice is a build error here rather than a
/// `ReferenceError` in a page. With no parameters - `script!("Measure")` - the
/// compiled expression is returned as it stands.
#[macro_export]
macro_rules! script {
	($Path:literal) => {
		include_str!(concat!(env!("OUT_DIR"), "/Script/", $Path, ".js"))
	};
	($Path:literal; $($Name:ident = $Value:expr),* $(,)?) => {{
		const PARAMETERS:&str = include_str!(concat!(env!("OUT_DIR"), "/Script/", $Path, ".params"));
		const _:() = $crate::AssertParameters(PARAMETERS, &[$(stringify!($Name)),*]);
		$crate::FillDeclared(
			include_str!(concat!(env!("OUT_DIR"), "/Script/", $Path, ".js")),
			PARAMETERS,
			&[$((stringify!($Name), AsRef::<str>::as_ref(&$Value))),*],
		)
	}};
}
