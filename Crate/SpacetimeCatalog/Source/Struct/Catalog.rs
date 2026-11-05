//! The catalog: the six tables of the agentic model, as data.
//!
//! A SpacetimeDB module compiles its tables into the published module, so a row
//! in a table-definition table cannot create one, and no reducer can execute
//! DDL. What can be shared instead is the description: name, columns, types,
//! keys, and whether a table is append-only. A module's `Schema.ts` renders its
//! `table(...)` declarations from this, and a reader diffs two versions of this
//! file to see what a generation changed.
//!
//! Times are whatever unit the module's clock uses, held as a signed integer.
//! The rules only add, compare and store them, so the unit belongs to the
//! module; a `ttl` passed to [`crate::Claim`] or [`crate::Renew`] must be in
//! that same unit.

use crate::Struct::{
	Column::{Column, ColumnType, Key},
	Table::{Table, TableName},
};

/// Every type a column may have, so a test can hold every column to the list.
pub const ColumnTypes:[ColumnType; 6] = ColumnType::All();

/// The six tables, in the order a reader meets them: what may be bound, what
/// was bound, who holds what, what was disclosed, what was inferred, what
/// happened.
pub static Tables:[Table; 6] = [
	Table {
		Name:TableName::Capability,
		AppendOnly:true,
		Columns:&[
			Column {
				Name:"id",
				Type:ColumnType::Text,
				Key:Some(Key::Primary),
				Note:"`<name>@<version>` - the identifier a manifest names",
			},
			Column {
				Name:"name",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"the stable capability name, without a version",
			},
			Column {
				Name:"version",
				Type:ColumnType::Text,
				Key:None,
				Note:"the version this declaration pins",
			},
			Column {
				Name:"state",
				Type:ColumnType::Text,
				Key:None,
				Note:"stateless | holds-transport | holds-identity | holds-workspace",
			},
			Column {
				Name:"declaredAt",
				Type:ColumnType::Number,
				Key:None,
				Note:"the context's now at declaration, which is the module's init",
			},
			Column {
				Name:"declaredBy",
				Type:ColumnType::Text,
				Key:None,
				Note:"who declared it, so a declaration has provenance",
			},
		],
	},
	Table {
		Name:TableName::Generation,
		AppendOnly:true,
		Columns:&[
			Column {
				Name:"id",
				Type:ColumnType::Text,
				Key:Some(Key::Primary),
				Note:"the sequence number as a decimal string: `1`, `2`, ...",
			},
			Column {
				Name:"sequence",
				Type:ColumnType::Number,
				Key:None,
				Note:"the same number as a number, so a successor's id needs no parsing",
			},
			Column {
				Name:"parent",
				Type:ColumnType::OptionalText,
				Key:None,
				Note:"the generation this one supersedes; null for the first",
			},
			Column {
				Name:"manifestDigest",
				Type:ColumnType::Text,
				Key:None,
				Note:"the composition root's name for the manifest it bound",
			},
			Column {
				Name:"capabilities",
				Type:ColumnType::JSON,
				Key:None,
				Note:"the capability ids the manifest names, sorted, as a JSON array",
			},
			Column {
				Name:"boundAt",
				Type:ColumnType::Number,
				Key:None,
				Note:"the context's now at bind",
			},
			Column { Name:"boundBy", Type:ColumnType::Text, Key:None, Note:"the sender at bind" },
		],
	},
	Table {
		Name:TableName::Lease,
		AppendOnly:false,
		Columns:&[
			Column {
				Name:"id",
				Type:ColumnType::Text,
				Key:Some(Key::Primary),
				Note:"the resource id, e.g. `browser:default` or `task:<id>`",
			},
			Column {
				Name:"generation",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"the generation the lease was granted under; a lease never adopts a later one",
			},
			Column {
				Name:"holder",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"the sender that holds it - the only sender a fenced write is accepted from",
			},
			Column {
				Name:"task",
				Type:ColumnType::Text,
				Key:None,
				Note:"the attempt the lease was granted for",
			},
			Column {
				Name:"fence",
				Type:ColumnType::Number,
				Key:None,
				Note:"incremented by every claim; a write carrying an older fence is refused",
			},
			Column {
				Name:"state",
				Type:ColumnType::Text,
				Key:None,
				Note:"held | released | quarantined - quarantined means an effect admitted under this epoch was \
				      refused when it landed, so the resource is not reassigned until `resolve` states whether it did",
			},
			Column {
				Name:"acquiredAt",
				Type:ColumnType::Number,
				Key:None,
				Note:"the context's now at the claim that opened this epoch",
			},
			Column {
				Name:"renewedAt",
				Type:ColumnType::OptionalNumber,
				Key:None,
				Note:"null until the first renew of this epoch",
			},
			Column {
				Name:"expiresAt",
				Type:ColumnType::Number,
				Key:None,
				Note:"acquiredAt + ttl; every fenced write compares it to now, because an expired lease cannot be \
				      renewed back to life",
			},
		],
	},
	Table {
		Name:TableName::Probe,
		AppendOnly:true,
		Columns:&[
			Column {
				Name:"id",
				Type:ColumnType::Text,
				Key:Some(Key::Primary),
				Note:"`<property>:<version>:<audience>` - the reveal is its own receipt, so a second probe at the \
				      same version finds this row",
			},
			Column {
				Name:"property",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"the property that was probed",
			},
			Column {
				Name:"version",
				Type:ColumnType::Number,
				Key:None,
				Note:"the property version the observation is pinned to; seeing the current truth is a separate \
				      entitlement",
			},
			Column {
				Name:"audience",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"the probe's sender - a caller-supplied audience would be a caller-supplied identity",
			},
			Column {
				Name:"observed",
				Type:ColumnType::Text,
				Key:None,
				Note:"what was disclosed, never the private value it was computed from",
			},
			Column {
				Name:"at",
				Type:ColumnType::Number,
				Key:None,
				Note:"the context's now at the reveal",
			},
			Column {
				Name:"generation",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"the generation the reveal was made under",
			},
		],
	},
	Table {
		Name:TableName::Belief,
		AppendOnly:false,
		Columns:&[
			Column {
				Name:"id",
				Type:ColumnType::Text,
				Key:Some(Key::Primary),
				Note:"`<audience>:<property>:<version>` - one current belief per audience",
			},
			Column {
				Name:"audience",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"the actor the belief belongs to, and the only actor it is about",
			},
			Column {
				Name:"property",
				Type:ColumnType::Text,
				Key:None,
				Note:"the property being inferred",
			},
			Column {
				Name:"version",
				Type:ColumnType::Number,
				Key:None,
				Note:"the property version the belief was formed at",
			},
			Column {
				Name:"interpretation",
				Type:ColumnType::Text,
				Key:None,
				Note:"the audience's own reading of its observations; never the private truth",
			},
			Column {
				Name:"derivedFrom",
				Type:ColumnType::Text,
				Key:None,
				Note:"the probe row it was derived from, so it can be rebuilt",
			},
			Column {
				Name:"at",
				Type:ColumnType::Number,
				Key:None,
				Note:"the context's now when the belief was derived",
			},
		],
	},
	Table {
		Name:TableName::Trace,
		AppendOnly:true,
		Columns:&[
			Column {
				Name:"id",
				Type:ColumnType::Text,
				Key:Some(Key::Primary),
				Note:"`<at>:<kind>:<subject>` - a replay at the same instant is the same fact",
			},
			Column {
				Name:"at",
				Type:ColumnType::Number,
				Key:None,
				Note:"the context's now when it was recorded",
			},
			Column {
				Name:"kind",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"what happened; a closed set the module owns",
			},
			Column {
				Name:"subject",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"what it happened to - a resource, a generation, a property",
			},
			Column {
				Name:"payload",
				Type:ColumnType::Text,
				Key:None,
				Note:"opaque JSON, so a new event shape needs no migration; never a private value",
			},
			Column {
				Name:"generation",
				Type:ColumnType::Text,
				Key:Some(Key::Index),
				Note:"the generation the event happened under",
			},
			Column {
				Name:"sender",
				Type:ColumnType::Text,
				Key:None,
				Note:"the sender that recorded it",
			},
		],
	},
];

/// The table names, in the order a reader meets them.
pub const TableNames:[TableName; 6] = TableName::All();

impl TableName {
	/// The table this name refers to.
	///
	/// Infallible, because [`Tables`] describes every name the type has: the
	/// two are one statement written twice, and the catalog test holds them
	/// together.
	pub fn Table(self) -> &'static Table {
		Tables
			.iter()
			.find(|Table| Table.Name == self)
			.expect("the catalog describes every table its type names")
	}
}

/// The column a row is keyed by, which is what `find`, `insert`, `update` and
/// `delete` address a row with.
///
/// It reads the description rather than assuming `id`, so a table whose key is
/// not called `id` cannot silently be keyed wrongly by a context that guesses.
pub fn PrimaryKey(Table:TableName) -> &'static str {
	Table.Table().Primary().expect("every table declares exactly one primary key")
}
