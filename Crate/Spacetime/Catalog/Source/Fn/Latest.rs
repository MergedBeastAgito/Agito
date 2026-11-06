//! The coherence predicate: the latest bound generation, or nothing.

use crate::Struct::{Context::Context, Row::Row, Table::TableName};

/// The latest bound generation, or `None` when `Generation` is unknown or
/// superseded.
///
/// A generation is superseded by the existence of its successor, not by a
/// column someone rewrote: ids are dense sequence numbers, so `sequence + 1`
/// being present is the whole test. That is one key lookup, which is the only
/// way to answer "is this the latest?" through a context that finds by key and
/// cannot scan - and it leaves the predecessor byte-identical, which is what
/// "appends a generation" has to mean if the binding history is to be
/// auditable.
///
/// The `sequence.to_string() == id` check is what makes the density assumption
/// fail closed: a row whose key disagrees with its own sequence number cannot
/// be extended safely, so it is treated as no generation at all rather than as
/// one.
pub fn Latest<C:Context>(Ctx:&C, Generation:&str) -> Option<Row> {
	let Row = Ctx.Find(TableName::Generation, Generation)?;
	let Sequence = Row.Number("sequence")?;
	if Sequence.to_string() != Generation {
		return None;
	}
	// A sequence at the top of the range has no successor to be superseded by,
	// so there is nothing to look up rather than a lookup that would wrap.
	let Some(Successor) = Sequence.checked_add(1) else {
		return Some(Row);
	};
	if Ctx.Find(TableName::Generation, &Successor.to_string()).is_some() {
		return None;
	}
	Some(Row)
}
