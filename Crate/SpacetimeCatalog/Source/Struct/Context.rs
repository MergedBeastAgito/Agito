//! The context a rule is given, and the whole of it.
//!
//! This is the seam that makes the rules testable without SpacetimeDB. The
//! consuming module's `ctx` has far more on it - an identity type, a timestamp
//! type, indexes, views, scheduling - and none of that belongs in a rule. What
//! a rule needs is five things: who is calling, what time it is, and a way to
//! read, insert, update and delete one row by key.
//!
//! Two decisions that look like omissions and are not:
//!
//! - [`Context::Sender`] is a string, not the module's identity type. An
//!   adapter renders `ctx.sender` once, at the boundary, and every rule
//!   compares that rendering. A rule that compared an identity to a
//!   caller-supplied argument would be comparing two different things.
//! - [`Context::Now`] is a number in whatever unit the module's clock uses, and
//!   no rule reads a wall clock. A reducer that read the system clock could not
//!   be replayed, and a test could not decide what time it is.
//!
//! [`Context::Delete`] is declared because the context has it. No rule in this
//! crate deletes a row; an in-memory context records a delete as a write and
//! refuses one on an append-only table, so a rule that started deleting history
//! would fail a test rather than pass review.
//!
//! # Why the three writes are infallible
//!
//! A rule answers a [`crate::Answer`] for everything a caller can get wrong,
//! and never for a write that did not land. A write defect is not an argument:
//! it is the rule or the adapter being wrong, and turning it into a refusal
//! would report a caller's mistake for a corruption. So a context panics on one
//! (a row already at its key, an update of an append-only table) exactly as the
//! reference fake throws, and the case
//! `every_rule_answers_a_malformed_argument_with_a_refusal_and_writes_nothing`
//! proves the malformed path never reaches a write.

use crate::Struct::{Row::Row, Table::TableName};

/// What a rule is given: the caller, the time, and one row at a time by key.
pub trait Context {
	/// The caller. The recipient of a reveal, the holder of a lease, the author
	/// of a trace - never an identity supplied in an argument.
	fn Sender(&self) -> &str;

	/// The module's time for this call. Every timestamp a row carries comes
	/// from here.
	fn Now(&self) -> i64;

	/// The row keyed by the table's primary column, or `None`. One lookup, no
	/// scan.
	fn Find(&self, Table:TableName, Id:&str) -> Option<Row>;

	/// Insert a row. A row already at its key is a defect, not an overwrite.
	fn Insert(&mut self, Table:TableName, Row:Row);

	/// Replace the row at the key it carries. A missing row is a defect.
	fn Update(&mut self, Table:TableName, Row:Row);

	/// Remove the row at `Id`. No rule in this crate calls it.
	fn Delete(&mut self, Table:TableName, Id:&str);
}
