//! Whether a text names a token, as a whole name.
//!
//! A substring test is not enough: `Notes.md` is a substring of `Notes.md.bak`,
//! so an index could appear to name a file it never names. The boundaries are
//! the characters a file or directory name is built from, which is what the
//! TypeScript implementation spells as `(^|[^\w.-])token(?![\w.-])` - and this
//! is that rule, written without a regular-expression dependency so the crate
//! keeps its dependency list to a digest and a serialiser.

/// Whether a character may continue a file or directory name.
fn IsName(Character:char) -> bool { Character.is_alphanumeric() || matches!(Character, '_' | '.' | '-') }

/// Whether `Text` names `Token` between two name boundaries.
pub fn Match(Text:&str, Token:&str) -> bool {
	if Token.is_empty() {
		return false;
	}
	for (At, _) in Text.char_indices() {
		if !Text[At..].starts_with(Token) {
			continue;
		}
		let Before = Text[..At].chars().next_back();
		let After = Text[At + Token.len()..].chars().next();
		if !Before.is_some_and(IsName) && !After.is_some_and(IsName) {
			return true;
		}
	}
	false
}
