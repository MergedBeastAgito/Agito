//! Renew a live lease.
//!
//! A stale fence, an expired lease and another holder's lease are all refused,
//! and none of them writes. Renewal extends; it never resurrects. An expired
//! lease is re-claimed, which increments the fence and therefore invalidates
//! the worker that was late.

use crate::{
	Fn::Fenced::Fenced,
	Struct::{Answer::Answer, Code::Code, Context::Context, Renew::Renew as RenewArgs, Row::Value, Table::TableName},
};

/// Renew a lease the caller already holds.
pub fn Renew<C:Context>(Ctx:&mut C, Args:RenewArgs) -> Answer {
	let RenewArgs { Resource, Generation, Fence, Ttl } = Args;
	if Ttl <= 0 {
		return Answer::Refused(Code::AdmissionRefused);
	}
	let Some(ExpiresAt) = Ctx.Now().checked_add(Ttl) else {
		return Answer::Refused(Code::AdmissionRefused);
	};
	let Some(Lease) = Fenced(Ctx, &Generation, &Resource, Fence) else {
		return Answer::Refused(Code::AdmissionRefused);
	};
	let Now = Ctx.Now();
	Ctx.Update(
		TableName::Lease,
		Lease
			.With("renewedAt", Value::Number(Now))
			.With("expiresAt", Value::Number(ExpiresAt)),
	);
	Answer::Accepted
}
