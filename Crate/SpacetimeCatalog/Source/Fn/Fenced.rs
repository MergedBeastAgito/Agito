//! The two questions every fenced write asks: is this lease live, and does this
//! caller hold it at this fence under this generation?

use crate::{
	Fn::Latest::Latest,
	Struct::{Context::Context, Row::Row, Table::TableName},
};

/// Whether a lease row is live at `Now`.
///
/// Expiry is a comparison, never a stored transition: a row that says `held`
/// but whose deadline has passed is expired, because no timer has to have run
/// for that to be true. This is also why an expired lease cannot be renewed
/// back to life - [`Fenced`] asks this question before any write.
pub(crate) fn IsLive(Lease:&Row, Now:i64) -> bool {
	Lease.Text("state") == Some("held") && Lease.Number("expiresAt").is_some_and(|ExpiresAt| ExpiresAt > Now)
}

/// The lease a fenced write is admitted under, or `None` when it is not.
///
/// Every condition is a refusal with the same reason on purpose: a caller
/// learns that its write did not land, not which of six things was wrong with
/// it. The fence is checked before expiry because a stale fence is the ordinary
/// case - the worker that woke up late - and the answer must not depend on how
/// late.
pub(crate) fn Fenced<C:Context>(Ctx:&C, Generation:&str, Resource:&str, Fence:i64) -> Option<Row> {
	Latest(Ctx, Generation)?;
	let Lease = Ctx.Find(TableName::Lease, Resource)?;
	if Lease.Text("holder") != Some(Ctx.Sender()) {
		return None;
	}
	if Lease.Text("state") != Some("held") {
		return None;
	}
	if Lease.Text("generation") != Some(Generation) {
		return None;
	}
	if Lease.Number("fence") != Some(Fence) {
		return None;
	}
	if !IsLive(&Lease, Ctx.Now()) {
		return None;
	}
	Some(Lease)
}
