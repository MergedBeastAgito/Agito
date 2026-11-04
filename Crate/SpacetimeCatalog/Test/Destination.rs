//! The destination's rule, as the reducer makes it - and the mirror of the Rust
//! late-arrival test.
//!
//! `Crate/Runtime/Test/Runtime.rs` runs the same transcript against the local
//! stand-in:
//!
//!   claim a resource
//!   admit an effect
//!   let the lease expire while the effect is in flight
//!   let the effect perform its external action
//!   present the record
//!
//! and asserts the same thing this file asserts: the record is refused, the
//! effect is never counted as a success, the refusal is written as an
//! ambiguity, and the resource is not reassigned until a resolver says what
//! happened. Two implementations, one answer. If they ever disagree, one of
//! them is wrong and the other is a second authority.
//!
//! The vocabulary matches too: this side refuses with `admission-refused`,
//! which is what `Code::AdmissionRefused` names on the Rust side, and both
//! manifests declare that code.

#[path = "Fake.rs"]
mod Fake;

use AgenticSpacetimeCatalog::{
	Answer,
	Bind,
	BindArgs,
	Claim,
	ClaimArgs,
	Code,
	Context,
	Effect,
	Manifest,
	Outcome,
	Record,
	Resolve,
	ResolveArgs,
	Row,
	TableName,
};
use Fake::{Memory, Write};

const BROWSE:&str = "agentic.browse@0.1.0";
const RESOURCE:&str = "browser:default";

/// A database with one capability declared and generation 1 bound.
fn Catalog(Sender:&str, Now:i64) -> Memory {
	let mut Ctx = Memory::New(Sender, Now);
	let At = BROWSE.find('@').expect("a capability id names a version");
	let DeclaredAt = Ctx.Now();
	Ctx.Insert(
		TableName::Capability,
		Row::Of([
			("id", BROWSE.into()),
			("name", BROWSE[..At].into()),
			("version", BROWSE[At + 1..].into()),
			("state", "stateless".into()),
			("declaredAt", DeclaredAt.into()),
			("declaredBy", "module".into()),
		]),
	);
	assert_eq!(
		Bind(
			&mut Ctx,
			BindArgs {
				Previous:None,
				Manifest:Manifest { Digest:"sha256:first".to_string(), Capabilities:vec![BROWSE.to_string()] },
			}
		),
		Answer::Accepted
	);
	Ctx
}

/// A claim on the default resource.
fn Claiming() -> ClaimArgs {
	ClaimArgs {
		Resource:RESOURCE.to_string(),
		Generation:"1".to_string(),
		Task:"task:1".to_string(),
		Ttl:1_000,
	}
}

/// An effect asking to be recorded at the first fence.
fn Effecting() -> Effect {
	Effect {
		Generation:"1".to_string(),
		Resource:RESOURCE.to_string(),
		Fence:1,
		Name:"write".to_string(),
	}
}

/// The writes made since `Mark`, which is how "wrote nothing" is asserted.
fn Written(Ctx:&Memory, Mark:usize) -> &[Write] { &Ctx.Writes[Mark..] }

/// Every write as the pair an assertion reads.
fn Wrote(Writes:&[Write]) -> Vec<(&str, TableName)> {
	Writes.iter().map(|Write| (Write.Kind.Name(), Write.Table)).collect()
}

/// The kinds of every trace row, in the order they were written.
fn Kinds(Ctx:&Memory) -> Vec<String> {
	Ctx.Rows(TableName::Trace)
		.iter()
		.map(|Row| Row.Text("kind").unwrap_or_default().to_string())
		.collect()
}

/// One column of one row, as the string it holds.
fn Text(Row:&Row, Column:&str) -> String { Row.Text(Column).unwrap_or_default().to_string() }

#[test]
fn an_effect_that_outlives_its_lease_is_refused_where_it_lands_and_recorded_ambiguous() {
	// The Rust transcript, step for step. The external action is unobservable
	// from here - that is the whole point of an effect - so what this side
	// can prove is the other half: the record was refused, the effect was
	// never counted, and the refusal is not silence.
	let mut Ctx = Catalog("agent:a", 0);
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.Now = 1_001;
	let Mark = Ctx.Writes.len();
	assert_eq!(Record(&mut Ctx, Effecting()), Answer::Refused(Code::AdmissionRefused));

	// Two writes, and neither of them is the effect: the ambiguity, then the
	// quarantine. A refused effect leaves nothing in the effect's own history.
	let Writes = Written(&Ctx, Mark);
	assert_eq!(Wrote(Writes), vec![("insert", TableName::Trace), ("update", TableName::Lease)]);
	let Ambiguity = Writes[0].Row.as_ref().expect("the ambiguity row");
	let Quarantine = Writes[1].Row.as_ref().expect("the quarantined lease row");
	assert_eq!(Text(Ambiguity, "kind"), "effect-ambiguous");
	assert_eq!(Text(Quarantine, "state"), "quarantined");
	assert_eq!(
		Text(Ambiguity, "payload"),
		"{\"name\":\"write\",\"fence\":1,\"reason\":\"outlived its authority\"}"
	);

	// The resource is not reassigned while the outcome is unknown.
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Refused(Code::LeaseHeld));
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE)
			.and_then(|Row| Row.Text("state").map(str::to_string)),
		Some("quarantined".to_string())
	);

	// A resolver states what happened, and only then is reassignment possible.
	assert_eq!(
		Resolve(
			&mut Ctx,
			ResolveArgs {
				Resource:RESOURCE.to_string(),
				Generation:"1".to_string(),
				Outcome:Outcome::Landed
			}
		),
		Answer::Accepted
	);
	let Resolved = Ctx
		.Rows(TableName::Trace)
		.into_iter()
		.find(|Row| Row.Text("kind") == Some("effect-resolved"))
		.expect("the resolution row");
	assert_eq!(Text(&Resolved, "payload"), "{\"outcome\":\"landed\",\"fence\":1}");
	// The ambiguity stays. A resolution that erased the question would be the
	// retraction the handover rule forbids.
	assert_eq!(Kinds(&Ctx).iter().filter(|Kind| *Kind == "effect-ambiguous").count(), 1);

	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE).and_then(|Row| Row.Number("fence")),
		Some(2)
	);
	// Across the whole transcript, the effect was never recorded as a success.
	assert_eq!(Kinds(&Ctx).iter().filter(|Kind| *Kind == "effect").count(), 0);
}

#[test]
fn a_caller_that_reproduces_the_rows_numbers_is_still_not_the_holder() {
	// The review's first gap. Every number this caller presents is correct -
	// the generation, the resource, the fence - and it is refused, because
	// the row says who holds it and the transport says who is asking.
	let mut Ctx = Catalog("agent:a", 0);
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.As("agent:b");
	let Mark = Ctx.Writes.len();
	assert_eq!(Record(&mut Ctx, Effecting()), Answer::Refused(Code::AdmissionRefused));
	// Nothing was admitted for this caller, so there is no external action to
	// be ambiguous about, and the refusal writes nothing at all.
	assert!(Written(&Ctx, Mark).is_empty());
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE)
			.and_then(|Row| Row.Text("state").map(str::to_string)),
		Some("held".to_string())
	);
	assert_eq!(Kinds(&Ctx).iter().filter(|Kind| *Kind == "effect-ambiguous").count(), 0);
}

#[test]
fn an_effect_from_a_superseded_fence_is_refused_without_an_ambiguity() {
	// A superseded epoch is where the takeover, not the destination, does the
	// accounting: the row no longer describes that epoch, so the rule refuses
	// and says nothing about an effect it cannot attribute.
	let mut Ctx = Catalog("agent:a", 0);
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.Now = 1_001;
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE).and_then(|Row| Row.Number("fence")),
		Some(2)
	);
	let Mark = Ctx.Writes.len();
	assert_eq!(Record(&mut Ctx, Effecting()), Answer::Refused(Code::AdmissionRefused));
	assert!(Written(&Ctx, Mark).is_empty());
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE)
			.and_then(|Row| Row.Text("state").map(str::to_string)),
		Some("held".to_string())
	);
}

#[test]
fn an_effect_whose_lease_was_released_while_it_ran_is_refused_and_recorded_ambiguous() {
	// Release is the other way authority ends. The catalog has no `release`
	// reducer, so the row is moved here the way `Resolve` moves it: the
	// rule reads the state, not the call that set it.
	let mut Ctx = Catalog("agent:a", 0);
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	let Lease = Ctx.Find(TableName::Lease, RESOURCE).expect("the lease row");
	Ctx.Update(TableName::Lease, Lease.With("state", "released".into()));
	let Mark = Ctx.Writes.len();
	assert_eq!(Record(&mut Ctx, Effecting()), Answer::Refused(Code::AdmissionRefused));
	let Writes = Written(&Ctx, Mark);
	let Ambiguity = Writes[0].Row.as_ref().expect("the ambiguity row");
	assert_eq!(Text(Ambiguity, "kind"), "effect-ambiguous");
	assert_eq!(
		Text(Ambiguity, "payload"),
		"{\"name\":\"write\",\"fence\":1,\"reason\":\"released while the effect was in flight\"}"
	);
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE)
			.and_then(|Row| Row.Text("state").map(str::to_string)),
		Some("quarantined".to_string())
	);
}

#[test]
fn an_effect_presented_while_its_lease_is_live_is_recorded_not_quarantined() {
	let mut Ctx = Catalog("agent:a", 0);
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	let Mark = Ctx.Writes.len();
	assert_eq!(Record(&mut Ctx, Effecting()), Answer::Accepted);
	let Writes = Written(&Ctx, Mark);
	assert_eq!(Wrote(Writes), vec![("insert", TableName::Trace)]);
	let Receipt = Writes[0].Row.as_ref().expect("the effect row");
	assert_eq!(Text(Receipt, "kind"), "effect");
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE)
			.and_then(|Row| Row.Text("state").map(str::to_string)),
		Some("held".to_string()),
		"a recorded effect changes no lease"
	);
	assert_eq!(Kinds(&Ctx).iter().filter(|Kind| *Kind == "effect-ambiguous").count(), 0);
}
