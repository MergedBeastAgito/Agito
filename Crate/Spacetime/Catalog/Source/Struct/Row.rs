//! One row, and the values a column may hold.
//!
//! A row is keyed by column name, and the catalog in [`crate::Tables`]
//! describes it. The values are the four a column type admits: a string, a
//! number, a flag, and `Null` for a column that is absent rather than empty.

use std::collections::BTreeMap;

/// Every value a column may hold.
///
/// `Null` is how an optional column is absent. It is not the empty string and
/// not zero: a rule that compared an absent expiry to zero would treat a lease
/// with no deadline as an expired one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
	/// A string, which is also how a JSON document is held.
	Text(String),
	/// A whole number: a count, a timestamp, a fence or a version.
	Number(i64),
	/// A flag.
	Boolean(bool),
	/// Absent.
	Null,
}

impl From<&str> for Value {
	fn from(Text:&str) -> Value { Value::Text(Text.to_string()) }
}

impl From<String> for Value {
	fn from(Text:String) -> Value { Value::Text(Text) }
}

impl From<i64> for Value {
	fn from(Number:i64) -> Value { Value::Number(Number) }
}

impl From<bool> for Value {
	fn from(Flag:bool) -> Value { Value::Boolean(Flag) }
}

/// One row, keyed by column name.
///
/// A map rather than a struct per table: a row is the thing a context reads,
/// inserts and updates by key, and the catalog is the only place its columns
/// are stated. A struct per table would be a second statement of the same
/// columns, and the two would drift.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row {
	Values:BTreeMap<String, Value>,
}

impl Row {
	/// A row from the columns it carries.
	///
	/// Every described column must be named - a missing one is refused by the
	/// context that checks the row, not silently stored as absent - so a rule
	/// states the whole row and a reader sees it whole.
	pub fn Of<const N: usize>(Fields:[(&str, Value); N]) -> Row {
		let mut Values = BTreeMap::new();
		for (Column, One) in Fields {
			Values.insert(Column.to_string(), One);
		}
		Row { Values }
	}

	/// The value at this column, if the row carries one.
	pub fn Get(&self, Column:&str) -> Option<&Value> { self.Values.get(Column) }

	/// The string at this column, if it holds one.
	pub fn Text(&self, Column:&str) -> Option<&str> {
		match self.Values.get(Column)? {
			Value::Text(Text) => Some(Text),
			_ => None,
		}
	}

	/// The number at this column, if it holds one.
	pub fn Number(&self, Column:&str) -> Option<i64> {
		match self.Values.get(Column)? {
			Value::Number(Number) => Some(*Number),
			_ => None,
		}
	}

	/// The same row with one column replaced, which is how a rule that rewrites
	/// one field keeps the rest of the row byte-identical.
	pub fn With(mut self, Column:&str, Value:Value) -> Row {
		self.Set(Column, Value);
		self
	}

	/// Replace one column in place.
	pub fn Set(&mut self, Column:&str, Value:Value) -> &mut Row {
		self.Values.insert(Column.to_string(), Value);
		self
	}

	/// Every column the row carries, in name order.
	pub fn Entries(&self) -> impl Iterator<Item = (&str, &Value)> {
		self.Values.iter().map(|(Column, Value)| (Column.as_str(), Value))
	}

	/// How many columns the row carries.
	pub fn Len(&self) -> usize { self.Values.len() }

	/// Whether the row carries no columns at all.
	pub fn IsEmpty(&self) -> bool { self.Values.is_empty() }
}
