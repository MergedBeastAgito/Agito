//! A context in memory, with three properties a real one has and a test needs.
//!
//! 1. **It records every write.** A refusal that still wrote is the failure the
//!    fence and coherence rules exist to prevent, so the assertion is usually
//!    "and the fake recorded nothing".
//! 2. **It refuses to rewrite history.** A table the catalog marks append-only
//!    cannot be updated or deleted here, so "the previous generation is
//!    untouched" is enforced rather than asserted.
//! 3. **It checks every row against the catalog.** A write must name described
//!    columns, all of them, with the described types. A private value therefore
//!    cannot enter the database through a column the catalog never declared -
//!    and a rule that mistyped a column fails a test instead of passing review.
//!
//! The clock is `Now`, which the test moves. Nothing reads a wall clock, so a
//! sequence of reducer calls at a chosen time is a transcript the test controls
//! completely.
//!
//! It is the instrument, not the model: it lives beside the tests rather than
//! in `Source/`, because a module's adapter is the only context that ships.

// The whole seam is implemented, including `Delete`, which no rule calls: the
// context has it, and a fake that omitted it would be a narrower seam than the one
// a module satisfies.
#![allow(dead_code)]

use std::collections::BTreeMap;

use AgenticSpacetimeCatalog::{Context, PrimaryKey, Row, TableName, Value};

/// What one write did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteKind {
	/// A row was inserted.
	Insert,
	/// A row was replaced.
	Update,
	/// A row was removed.
	Delete,
}

impl WriteKind {
	/// The word a test's assertion reads.
	pub const fn Name(self) -> &'static str {
		match self {
			WriteKind::Insert => "insert",
			WriteKind::Update => "update",
			WriteKind::Delete => "delete",
		}
	}
}

/// One recorded write, in the order it happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Write {
	/// What it did.
	pub Kind:WriteKind,
	/// The table it did it to.
	pub Table:TableName,
	/// The key of the row it addressed.
	pub Id:String,
	/// The row, or `None` for a delete.
	pub Row:Option<Row>,
}

/// A context in memory: one map per table, and every write kept.
pub struct Memory {
	/// The caller. `As` changes it, as a second agent would.
	pub Sender:String,
	/// The time. The test moves it, and no rule reads any other.
	pub Now:i64,
	/// Every write, in the order the rules made them.
	pub Writes:Vec<Write>,
	Tables:BTreeMap<TableName, BTreeMap<String, Row>>,
}

impl Memory {
	/// A database with no rows, read as `Sender` at `Now`.
	pub fn New(Sender:&str, Now:i64) -> Memory {
		Memory { Sender:Sender.to_string(), Now, Writes:Vec::new(), Tables:BTreeMap::new() }
	}

	/// Become another caller on the same database, as a second agent would be.
	pub fn As(&mut self, Sender:&str) -> &mut Memory {
		self.Sender = Sender.to_string();
		self
	}

	/// Every row of one table, in the order it was written.
	pub fn Rows(&self, Table:TableName) -> Vec<Row> {
		self.Tables
			.get(&Table)
			.map(|Rows| Rows.values().cloned().collect())
			.unwrap_or_default()
	}

	/// The id a row is keyed by, read from the catalog rather than assumed.
	fn Id(Table:TableName, Row:&Row) -> String {
		let Key = PrimaryKey(Table);
		match Row.Get(Key) {
			Some(Value::Text(Id)) => Id.clone(),
			_ => panic!("{}.{Key} is not a string key", Table.Name()),
		}
	}
}

/// Check every row against the catalog before it is stored.
fn Check(Table:TableName, Row:&Row) {
	let Described = Table.Table();
	for (Name, Value) in Row.Entries() {
		let Some(Column) = Described.Column(Name) else {
			panic!("{}.{Name} is not a described column", Table.Name());
		};
		if !Column.Type.Accepts(Value) {
			panic!("{}.{} is {}, got {Value:?}", Table.Name(), Column.Name, Column.Type.Name());
		}
	}
	for Column in Described.Columns {
		if Row.Get(Column.Name).is_none() {
			panic!("{}.{} is missing from the row", Table.Name(), Column.Name);
		}
	}
}

impl Context for Memory {
	fn Sender(&self) -> &str { &self.Sender }

	fn Now(&self) -> i64 { self.Now }

	fn Find(&self, Table:TableName, Id:&str) -> Option<Row> {
		self.Tables.get(&Table).and_then(|Rows| Rows.get(Id)).cloned()
	}

	fn Insert(&mut self, Table:TableName, Row:Row) {
		Check(Table, &Row);
		let Id = Self::Id(Table, &Row);
		{
			let Rows = self.Tables.entry(Table).or_default();
			if Rows.contains_key(&Id) {
				panic!("insert of an existing row: {}/{Id}", Table.Name());
			}
			Rows.insert(Id.clone(), Row.clone());
		}
		self.Writes.push(Write { Kind:WriteKind::Insert, Table, Id, Row:Some(Row) });
	}

	fn Update(&mut self, Table:TableName, Row:Row) {
		if Table.Table().AppendOnly {
			panic!("update of an append-only table: {}", Table.Name());
		}
		Check(Table, &Row);
		let Id = Self::Id(Table, &Row);
		{
			let Rows = self.Tables.entry(Table).or_default();
			if !Rows.contains_key(&Id) {
				panic!("update of a missing row: {}/{Id}", Table.Name());
			}
			Rows.insert(Id.clone(), Row.clone());
		}
		self.Writes.push(Write { Kind:WriteKind::Update, Table, Id, Row:Some(Row) });
	}

	fn Delete(&mut self, Table:TableName, Id:&str) {
		if Table.Table().AppendOnly {
			panic!("delete from an append-only table: {}", Table.Name());
		}
		if self.Tables.entry(Table).or_default().remove(Id).is_none() {
			panic!("delete of a missing row: {}/{Id}", Table.Name());
		}
		self.Writes
			.push(Write { Kind:WriteKind::Delete, Table, Id:Id.to_string(), Row:None });
	}
}
