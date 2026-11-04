//! One table: its name, whether a row may ever be rewritten, and its columns.
//!
//! `AppendOnly` is a rule rather than a storage detail. The in-memory context a
//! test uses refuses an update or a delete on a table marked append-only here,
//! so "the previous generation is untouched" is enforced by the same data a
//! module would render - not by a comment.

use crate::Struct::Column::Column;

/// The six tables of the model, in the order a reader meets them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TableName {
	/// What may be bound: the capabilities a manifest may name.
	Capability,
	/// What was bound: every generation, and the manifest it pinned.
	Generation,
	/// Who holds what: one row per resource, current state.
	Lease,
	/// What was disclosed: one reveal per audience and property version.
	Probe,
	/// What was inferred: an audience's own reading of its observations.
	Belief,
	/// What happened: the append-only log, keyed by content.
	Trace,
}

impl TableName {
	/// The string the catalog and a module's declaration use.
	pub const fn Name(self) -> &'static str {
		match self {
			TableName::Capability => "capability",
			TableName::Generation => "generation",
			TableName::Lease => "lease",
			TableName::Probe => "probe",
			TableName::Belief => "belief",
			TableName::Trace => "trace",
		}
	}

	/// Every table, in the order a reader meets them.
	pub const fn All() -> [TableName; 6] {
		[
			TableName::Capability,
			TableName::Generation,
			TableName::Lease,
			TableName::Probe,
			TableName::Belief,
			TableName::Trace,
		]
	}

	/// The table a name refers to, if the catalog describes it.
	///
	/// A closed enum rather than a lookup that can miss: an undescribed table
	/// is a defect, and this is the type in which that defect cannot be
	/// expressed. A caller holding a string - a module reading its own
	/// argument, a harness reading a fixture - gets `None` here instead of
	/// a table that is not there.
	pub fn Parse(Value:&str) -> Option<TableName> { TableName::All().into_iter().find(|Name| Name.Name() == Value) }
}

/// One table's description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Table {
	/// The table's name, which is also its key in the catalog.
	pub Name:TableName,
	/// True when a row may be inserted but never updated or deleted.
	pub AppendOnly:bool,
	/// The columns, in the order a module would declare them.
	pub Columns:&'static [Column],
}

impl Table {
	/// The column with this name, if the table describes it.
	pub fn Column(&self, Name:&str) -> Option<&'static Column> {
		self.Columns.iter().find(|Column| Column.Name == Name)
	}

	/// The one column a row is keyed by, if the table declares exactly one.
	pub fn Primary(&self) -> Option<&'static str> {
		let mut Primary = self
			.Columns
			.iter()
			.filter(|Column| Column.Key == Some(crate::Struct::Column::Key::Primary));
		let First = Primary.next()?;
		if Primary.next().is_some() {
			return None;
		}
		Some(First.Name)
	}
}
