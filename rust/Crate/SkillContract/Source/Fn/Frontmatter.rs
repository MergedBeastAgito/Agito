//! The frontmatter block and the fields this port promises from it.
//!
//! A YAML parser would be a dependency, and the two fields a loader reads do not
//! justify one. What is read: top-level `key: value` pairs, a value quoted or
//! plain, and a `|` or `>` block scalar, which is how a long description is
//! usually written. What is not read: nested mappings and sequences. Those are
//! skipped rather than refused, because a skill may carry metadata this port does
//! not promise and a validator that rejects unfamiliar keys is a validator people
//! turn off.

use std::collections::BTreeMap;

/// The frontmatter block and the fields this port promises from it.
///
/// `Ok` is the mapping the block described, and may be empty. `Err` is the reason
/// the block did not describe one, worded so `validate` can paste it into a
/// finding without rewriting it.
pub type Frontmatter = Result<BTreeMap<String, String>, String>;

/// The YAML block at the top of a file. A file without one has nothing to read.
pub fn Split(Text: &str) -> Frontmatter {
	let Normalised = Text.replace("\r\n", "\n");
	let Lines: Vec<&str> = Normalised.split('\n').collect();
	if Lines.first().map(|Line| Line.trim()) != Some("---") {
		return Err("there is no frontmatter block".to_string());
	}
	let Some(End) = Lines.iter().skip(1).position(|Line| *Line == "---") else {
		return Err("the frontmatter block is never closed".to_string());
	};
	Parse(&Lines[1..End + 1])
}

/// A `key: value` pair, or why the line is not one.
fn Pair(Line: &str) -> Option<(&str, &str)> {
	let Colon = Line.find(':')?;
	let Key = &Line[..Colon];
	if Key.is_empty() {
		return None;
	}
	if !Key
		.chars()
		.all(|Character| Character.is_ascii_alphanumeric() || matches!(Character, '_' | '.' | '-'))
	{
		return None;
	}
	Some((Key, &Line[Colon + 1..]))
}

/// Whether a value opens a block scalar, and whether it keeps its line breaks.
fn Block(Value: &str) -> Option<bool> {
	let First = Value.chars().next()?;
	if First != '|' && First != '>' {
		return None;
	}
	let Rest = &Value[First.len_utf8()..];
	if !Rest.is_empty() && Rest != "+" && Rest != "-" {
		return None;
	}
	Some(First == '|')
}

/// The number of leading whitespace bytes, which is the block's own indentation.
fn Indent(Line: &str) -> usize {
	Line.len() - Line.trim_start().len()
}

/// A `|` or `>` block scalar with the block's own indentation removed.
///
/// A folded scalar joins its lines into one, a literal one keeps the breaks. Both
/// drop blank lines, because a description is a sentence rather than a document
/// and the port reads exactly one field this way.
fn Scalar(Lines: &[&str], Literal: bool) -> String {
	let Indented: Vec<&str> = Lines
		.iter()
		.copied()
		.filter(|Line| !Line.trim().is_empty())
		.collect();
	let Some(Indent) = Indented.iter().map(|Line| Indent(Line)).min() else {
		return String::new();
	};
	let Stripped: Vec<&str> = Lines
		.iter()
		.map(|Line| &Line[Indent.min(Line.len())..])
		.collect();
	if Literal {
		Stripped.join("\n").trim().to_string()
	} else {
		Stripped
			.iter()
			.filter(|Line| !Line.trim().is_empty())
			.copied()
			.collect::<Vec<&str>>()
			.join(" ")
			.trim()
			.to_string()
	}
}

/// A value with one matching pair of surrounding quotes removed.
fn Unquote(Value: &str) -> String {
	let Bytes = Value.as_bytes();
	if Bytes.len() >= 2 {
		let First = Bytes[0];
		if (First == b'"' || First == b'\'') && Bytes[Bytes.len() - 1] == First {
			return Value[1..Value.len() - 1].to_string();
		}
	}
	Value.to_string()
}

/// The pairs of a frontmatter block.
fn Parse(Lines: &[&str]) -> Frontmatter {
	let mut Data = BTreeMap::new();
	let mut Index = 0;
	while Index < Lines.len() {
		let Line = Lines[Index];
		Index += 1;
		let Trimmed = Line.trim();
		if Trimmed.is_empty() || Trimmed.starts_with('#') {
			continue;
		}
		let Some((Key, Value)) = Pair(Line) else {
			if Line.starts_with(char::is_whitespace) {
				continue;
			}
			return Err(format!(
				"{} is not a key: value pair",
				serde_json::to_string(Line).unwrap_or_else(|_| format!("{Line:?}"))
			));
		};
		let Value = Value.trim();
		if let Some(Literal) = Block(Value) {
			let mut Collected: Vec<&str> = Vec::new();
			while Index < Lines.len() {
				let Next = Lines[Index];
				if !Next.trim().is_empty() && !Next.starts_with(char::is_whitespace) {
					break;
				}
				Collected.push(Next);
				Index += 1;
			}
			Data.insert(Key.to_string(), Scalar(&Collected, Literal));
			continue;
		}
		Data.insert(Key.to_string(), Unquote(Value));
	}
	Ok(Data)
}
