//! One column of one table: its name, the type a module declaration renders,
//! the key it carries, and what it means.
//!
//! The type list is data rather than an enum closed at the call site, because a
//! test asserts that every column uses one of the declared types - which is
//! what makes a typo in a column a failure rather than a silently unrenderable
//! declaration.

use crate::Struct::Row::Value;

/// Every type a column may have.
///
/// The names are the ones a module's schema renders from: `string` is
/// `t.string()`, `number` is `t.u64()`, `json` is a `t.string()` holding the
/// document's text, and the two optional types take `.optional()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColumnType {
	/// A string.
	Text,
	/// A whole number: a count, a timestamp, a fence or a version.
	Number,
	/// A flag.
	Boolean,
	/// A JSON document held as text, so a new shape needs no migration.
	JSON,
	/// A string, or absent.
	OptionalText,
	/// A number, or absent.
	OptionalNumber,
}

impl ColumnType {
	/// The string a module declaration is rendered from.
	pub const fn Name(self) -> &'static str {
		match self {
			ColumnType::Text => "string",
			ColumnType::Number => "number",
			ColumnType::Boolean => "boolean",
			ColumnType::JSON => "json",
			ColumnType::OptionalText => "optional-string",
			ColumnType::OptionalNumber => "optional-number",
		}
	}

	/// Every type, so a test can hold every column to the list.
	pub const fn All() -> [ColumnType; 6] {
		[
			ColumnType::Text,
			ColumnType::Number,
			ColumnType::Boolean,
			ColumnType::JSON,
			ColumnType::OptionalText,
			ColumnType::OptionalNumber,
		]
	}

	/// The type a catalog string names, if it names one of the six.
	pub fn Parse(Value:&str) -> Option<ColumnType> { ColumnType::All().into_iter().find(|Type| Type.Name() == Value) }

	/// Whether a value is one this type may hold.
	///
	/// This is what an in-memory context checks a row against, so a private
	/// value cannot enter the database through a column the catalog never
	/// declared and a rule that mistyped a column fails a test instead of
	/// passing review.
	pub fn Accepts(self, Value:&Value) -> bool {
		match self {
			ColumnType::Text | ColumnType::JSON => matches!(Value, Value::Text(_)),
			ColumnType::Number => matches!(Value, Value::Number(_)),
			ColumnType::Boolean => matches!(Value, Value::Boolean(_)),
			ColumnType::OptionalText => matches!(Value, Value::Text(_) | Value::Null),
			ColumnType::OptionalNumber => matches!(Value, Value::Number(_) | Value::Null),
		}
	}
}

/// What a column is keyed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
	/// The key `find`, `insert`, `update` and `delete` address a row by.
	Primary,
	/// A key a module would index.
	Index,
}

impl Key {
	/// The string a module declaration is rendered from.
	pub const fn Name(self) -> &'static str {
		match self {
			Key::Primary => "primary",
			Key::Index => "index",
		}
	}
}

/// One column: the row object's key, the type rendered for it, and its meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Column {
	/// The column name, which is also the key of the row object.
	pub Name:&'static str,
	/// The value type a module declaration renders.
	pub Type:ColumnType,
	/// `primary` for the key a row is addressed by, `index` for a key a module
	/// would index, `None` for a plain column.
	pub Key:Option<Key>,
	/// What the column means, and the rule it carries when it carries one.
	pub Note:&'static str,
}
