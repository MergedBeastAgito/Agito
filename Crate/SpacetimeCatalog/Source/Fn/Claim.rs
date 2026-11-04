//! Claim a resource, or be told it is held.
//!
//! A live lease refuses the claim - `lease-held`, not a queue, because a queue
//! in the reducer would be a second source of ownership. An expired or released
//! one is claimed by incrementing the fence, which is the only thing that makes
//! the previous holder's writes stop landing: the row is never deleted, so the
//! fence never restarts at one.

use crate::{
	Fn::{Fenced::IsLive, Latest::Latest},
	Struct::{
		Answer::Answer,
		Claim::Claim as ClaimArgs,
		Code::Code,
		Context::Context,
		Row::{Row, Value},
		Table::TableName,
	},
};

/// Claim a resource for one attempt.
pub fn Claim<C:Context>(Ctx:&mut C, Args:ClaimArgs) -> Answer {
	let ClaimArgs { Resource, Generation, Task, Ttl } = Args;
	if Latest(Ctx, &Generation).is_none() {
		return Answer::Refused(Code::AdmissionRefused);
	}
	if Resource.is_empty() {
		return Answer::Refused(Code::AdmissionRefused);
	}
	if Ttl <= 0 {
		return Answer::Refused(Code::AdmissionRefused);
	}
	let Some(ExpiresAt) = Ctx.Now().checked_add(Ttl) else {
		return Answer::Refused(Code::AdmissionRefused);
	};
	let Existing = Ctx.Find(TableName::Lease, &Resource);
	if let Some(Lease) = &Existing {
		// A quarantined resource is not reassigned. An effect admitted under
		// its last epoch may have landed, and a new holder would act on
		// a state nobody knows; `Resolve` is the only way out, and it
		// is a decision somebody has to make.
		if Lease.Text("state") == Some("quarantined") {
			return Answer::Refused(Code::LeaseHeld);
		}
		if IsLive(Lease, Ctx.Now()) {
			return Answer::Refused(Code::LeaseHeld);
		}
	}
	let Fence = match Existing.as_ref().and_then(|Lease| Lease.Number("fence")) {
		None => 1,
		Some(Previous) => {
			match Previous.checked_add(1) {
				Some(Next) => Next,
				// A fence that cannot advance cannot fence: reassigning under the same
				// number would let the previous holder's writes keep landing.
				None => return Answer::Refused(Code::AdmissionRefused),
			}
		},
	};
	let Now = Ctx.Now();
	let Sender = Ctx.Sender().to_string();
	let Row = Row::Of([
		("id", Value::Text(Resource)),
		("generation", Value::Text(Generation)),
		("holder", Value::Text(Sender)),
		("task", Value::Text(Task)),
		("fence", Value::Number(Fence)),
		("state", Value::Text("held".to_string())),
		("acquiredAt", Value::Number(Now)),
		("renewedAt", Value::Null),
		("expiresAt", Value::Number(ExpiresAt)),
	]);
	if Existing.is_none() {
		Ctx.Insert(TableName::Lease, Row);
	} else {
		Ctx.Update(TableName::Lease, Row);
	}
	Answer::Accepted
}
