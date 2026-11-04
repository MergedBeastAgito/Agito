//! The invariants, each proved by a refusal that wrote nothing.
//!
//! The fake context records every write, so "refused" and "refused without
//! writing" are different assertions here: a rule that returned a refusal after
//! mutating a row would fail the second one. It also refuses to rewrite an
//! append-only table and checks every row against the catalog, so a rule cannot
//! quietly break either promise and still pass.
//!
//! The cases are the ones the TypeScript statement of these rules carried, case
//! for case, with two additions the port made deliberately and states in the
//! card: a claim at the end of the clock, and the catalog's own name parser.
//! Everything else is the same transcript with the same expected rows.

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
	Latest,
	Manifest,
	Probe,
	ProbeAnswer,
	ProbeArgs,
	RecordTrace,
	Renew,
	RenewArgs,
	Row,
	TableName,
	Trace,
	Value,
};
use Fake::{Memory, Write};

const BROWSE:&str = "agentic.browse@0.1.0";
const CHAT:&str = "agentic.chat@0.1.0";
const RESOURCE:&str = "browser:default";

/// A database whose catalog already declares two capabilities.
fn Catalog(Sender:&str, Now:i64) -> Memory {
	let mut Ctx = Memory::New(Sender, Now);
	let DeclaredAt = Ctx.Now();
	for Id in [BROWSE, CHAT] {
		let At = Id.find('@').expect("a capability id names a version");
		Ctx.Insert(
			TableName::Capability,
			Row::Of([
				("id", Id.into()),
				("name", Id[..At].into()),
				("version", Id[At + 1..].into()),
				("state", "stateless".into()),
				("declaredAt", DeclaredAt.into()),
				("declaredBy", "module".into()),
			]),
		);
	}
	Ctx
}

/// The writes made since `Mark`, which is how "wrote nothing" is asserted.
fn Written(Ctx:&Memory, Mark:usize) -> &[Write] { &Ctx.Writes[Mark..] }

/// Every write as the pair an assertion reads.
fn Wrote(Writes:&[Write]) -> Vec<(&str, TableName)> {
	Writes.iter().map(|Write| (Write.Kind.Name(), Write.Table)).collect()
}

/// Bind a generation and answer its id.
fn Bound(Ctx:&mut Memory, Previous:Option<&str>, Digest:&str) -> String {
	assert_eq!(
		Bind(
			Ctx,
			BindArgs {
				Previous:Previous.map(str::to_string),
				Manifest:Manifest { Digest:Digest.to_string(), Capabilities:vec![BROWSE.to_string()] },
			}
		),
		Answer::Accepted
	);
	match Previous {
		None => "1".to_string(),
		Some(Previous) => (Previous.parse::<i64>().expect("a generation id is a number") + 1).to_string(),
	}
}

/// A claim on the default resource at the default generation.
fn Claiming() -> ClaimArgs {
	ClaimArgs {
		Resource:RESOURCE.to_string(),
		Generation:"1".to_string(),
		Task:"task:1".to_string(),
		Ttl:1_000,
	}
}

/// A renewal of the default resource at the first fence.
fn Renewing() -> RenewArgs {
	RenewArgs { Resource:RESOURCE.to_string(), Generation:"1".to_string(), Fence:1, Ttl:1_000 }
}

/// A reveal of the default property version by the default resource's holder.
fn Probing() -> ProbeArgs {
	ProbeArgs {
		Generation:"1".to_string(),
		Resource:RESOURCE.to_string(),
		Fence:1,
		Property:"agentic.browse.reliability".to_string(),
		Version:3,
		Observed:"high".to_string(),
	}
}

/// One appended event at the default instant, kind and subject.
fn Tracing() -> Trace {
	Trace {
		Generation:"1".to_string(),
		Kind:"lease.claimed".to_string(),
		Subject:RESOURCE.to_string(),
		Payload:"{\"fence\":1}".to_string(),
	}
}

#[test]
fn bind_appends_the_first_generation_at_the_senders_time_and_writes_exactly_one_row() {
	let mut Ctx = Catalog("agent:a", 1_000);
	let Mark = Ctx.Writes.len();
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
	assert_eq!(
		Ctx.Rows(TableName::Generation),
		vec![Row::Of([
			("id", "1".into()),
			("sequence", 1.into()),
			("parent", Value::Null),
			("manifestDigest", "sha256:first".into()),
			("capabilities", format!("[\"{BROWSE}\"]").into()),
			("boundAt", 1_000.into()),
			("boundBy", "agent:a".into()),
		])]
	);
	assert_eq!(Wrote(Written(&Ctx, Mark)), vec![("insert", TableName::Generation)]);
}

#[test]
fn bind_refuses_a_manifest_that_names_a_capability_the_catalog_does_not_declare_and_writes_nothing() {
	let mut Ctx = Catalog("agent:a", 0);
	let Mark = Ctx.Writes.len();
	assert_eq!(
		Bind(
			&mut Ctx,
			BindArgs {
				Previous:None,
				Manifest:Manifest {
					Digest:"sha256:first".to_string(),
					Capabilities:vec![BROWSE.to_string(), "agentic.vote@1.0.0".to_string()],
				},
			}
		),
		Answer::Refused(Code::AdmissionRefused)
	);
	assert!(Written(&Ctx, Mark).is_empty());
	assert_eq!(Ctx.Find(TableName::Generation, "1"), None);
}

#[test]
fn bind_appends_a_successor_and_leaves_the_previous_generation_untouched() {
	let mut Ctx = Catalog("agent:a", 1_000);
	Bound(&mut Ctx, None, "sha256:first");
	let First = Ctx.Find(TableName::Generation, "1");
	assert!(First.is_some());
	Ctx.Now = 2_000;
	let Mark = Ctx.Writes.len();
	assert_eq!(
		Bind(
			&mut Ctx,
			BindArgs {
				Previous:Some("1".to_string()),
				Manifest:Manifest {
					Digest:"sha256:second".to_string(),
					Capabilities:vec![BROWSE.to_string(), CHAT.to_string()],
				},
			}
		),
		Answer::Accepted
	);
	assert_eq!(Ctx.Find(TableName::Generation, "1"), First);
	assert_eq!(
		Ctx.Rows(TableName::Generation)
			.iter()
			.map(|Row| Row.Text("id").unwrap_or_default().to_string())
			.collect::<Vec<_>>(),
		vec!["1", "2"]
	);
	assert_eq!(Wrote(Written(&Ctx, Mark)), vec![("insert", TableName::Generation)]);
	assert_eq!(
		Ctx.Find(TableName::Generation, "2")
			.and_then(|Row| Row.Text("parent").map(str::to_string)),
		Some("1".to_string())
	);
	assert_eq!(
		Ctx.Find(TableName::Generation, "2").and_then(|Row| Row.Number("boundAt")),
		Some(2_000)
	);
	assert_eq!(
		Ctx.Find(TableName::Generation, "2")
			.and_then(|Row| Row.Text("capabilities").map(str::to_string)),
		Some(format!("[\"{BROWSE}\",\"{CHAT}\"]"))
	);
}

#[test]
fn bind_refuses_a_predecessor_that_is_already_superseded_or_that_was_never_bound() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	Bound(&mut Ctx, Some("1"), "sha256:second");
	let Mark = Ctx.Writes.len();
	assert_eq!(
		Bind(
			&mut Ctx,
			BindArgs {
				Previous:Some("1".to_string()),
				Manifest:Manifest { Digest:"sha256:x".to_string(), Capabilities:vec![BROWSE.to_string()] },
			}
		),
		Answer::Refused(Code::AdmissionRefused)
	);
	assert_eq!(
		Bind(
			&mut Ctx,
			BindArgs {
				Previous:Some("9".to_string()),
				Manifest:Manifest { Digest:"sha256:x".to_string(), Capabilities:vec![BROWSE.to_string()] },
			}
		),
		Answer::Refused(Code::AdmissionRefused)
	);
	assert!(Written(&Ctx, Mark).is_empty());
	assert_eq!(
		Ctx.Rows(TableName::Generation)
			.iter()
			.map(|Row| Row.Text("id").unwrap_or_default().to_string())
			.collect::<Vec<_>>(),
		vec!["1", "2"]
	);
}

#[test]
fn a_claim_while_a_lease_is_live_is_lease_held_and_writes_nothing() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.Now = 999;
	let Mark = Ctx.Writes.len();
	assert_eq!(
		Claim(&mut Ctx, ClaimArgs { Task:"task:2".to_string(), ..Claiming() }),
		Answer::Refused(Code::LeaseHeld)
	);
	assert!(Written(&Ctx, Mark).is_empty());
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE)
			.and_then(|Row| Row.Text("task").map(str::to_string)),
		Some("task:1".to_string())
	);
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE).and_then(|Row| Row.Number("fence")),
		Some(1)
	);
}

#[test]
fn the_fence_a_claim_past_expiry_increments_it_and_the_first_fence_can_no_longer_renew() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, ClaimArgs { Ttl:1_000, ..Claiming() }), Answer::Accepted);
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE).and_then(|Row| Row.Number("fence")),
		Some(1)
	);

	Ctx.Now = 1_001;
	assert_eq!(
		Claim(&mut Ctx, ClaimArgs { Task:"task:2".to_string(), Ttl:1_000, ..Claiming() }),
		Answer::Accepted
	);
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE).and_then(|Row| Row.Number("fence")),
		Some(2)
	);

	let Mark = Ctx.Writes.len();
	assert_eq!(
		Renew(&mut Ctx, RenewArgs { Fence:1, ..Renewing() }),
		Answer::Refused(Code::AdmissionRefused)
	);
	assert!(Written(&Ctx, Mark).is_empty());
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE).and_then(|Row| Row.Number("fence")),
		Some(2)
	);
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE).and_then(|Row| Row.Number("expiresAt")),
		Some(2_001)
	);
}

#[test]
fn renew_extends_a_live_lease_at_the_senders_time() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.Now = 500;
	assert_eq!(Renew(&mut Ctx, RenewArgs { Ttl:2_000, ..Renewing() }), Answer::Accepted);
	let Lease = Ctx.Find(TableName::Lease, RESOURCE).expect("the lease row");
	assert_eq!(Lease.Number("renewedAt"), Some(500));
	assert_eq!(Lease.Number("expiresAt"), Some(2_500));
	assert_eq!(Lease.Number("acquiredAt"), Some(0));
	assert_eq!(Lease.Number("fence"), Some(1));
}

#[test]
fn an_expired_lease_cannot_be_renewed_back_to_life() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.Now = 1_000;
	let Mark = Ctx.Writes.len();
	assert_eq!(Renew(&mut Ctx, Renewing()), Answer::Refused(Code::AdmissionRefused));
	assert!(Written(&Ctx, Mark).is_empty());
	assert_eq!(
		Ctx.Find(TableName::Lease, RESOURCE).and_then(|Row| Row.Number("expiresAt")),
		Some(1_000)
	);
}

#[test]
fn a_lease_is_renewed_only_by_the_sender_that_holds_it() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.As("agent:b");
	let Mark = Ctx.Writes.len();
	assert_eq!(Renew(&mut Ctx, Renewing()), Answer::Refused(Code::AdmissionRefused));
	assert!(Written(&Ctx, Mark).is_empty());
}

#[test]
fn a_probe_writes_exactly_one_row_carrying_the_audience_and_the_property_version_not_the_private_value() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.Now = 10;
	// The private truth the owner computed the disclosure from. It is
	// deliberately never passed to the rule: a secret the database was
	// never told cannot be selected from it, and this is the canary that
	// says so.
	let Secret = "internal-reliability-0.97";
	let Mark = Ctx.Writes.len();
	let Answer = Probe(&mut Ctx, Probing());
	let Observation = Row::Of([
		("id", "agentic.browse.reliability:3:agent:a".into()),
		("property", "agentic.browse.reliability".into()),
		("version", 3.into()),
		("audience", "agent:a".into()),
		("observed", "high".into()),
		("at", 10.into()),
		("generation", "1".into()),
	]);
	assert_eq!(Answer, ProbeAnswer::Observed(Observation.clone()));
	assert_eq!(Ctx.Rows(TableName::Probe), vec![Observation]);
	assert_eq!(Wrote(Written(&Ctx, Mark)), vec![("insert", TableName::Probe)]);
	let Everything = [TableName::Generation, TableName::Lease, TableName::Probe, TableName::Trace]
		.iter()
		.flat_map(|Table| Ctx.Rows(*Table))
		.map(|Row| format!("{Row:?}"))
		.collect::<Vec<String>>()
		.join("\n");
	assert!(!Everything.contains(Secret), "the private value reached a row:\n{Everything}");
}

#[test]
fn a_second_probe_by_the_same_audience_at_the_same_property_version_is_budget_exhausted_and_writes_nothing() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.Now = 10;
	assert!(Probe(&mut Ctx, Probing()).Ok());
	let Mark = Ctx.Writes.len();
	Ctx.Now = 20;
	assert_eq!(
		Probe(&mut Ctx, ProbeArgs { Observed:"low".to_string(), ..Probing() }),
		ProbeAnswer::Refused(Code::BudgetExhausted)
	);
	assert!(Written(&Ctx, Mark).is_empty());
	assert_eq!(Ctx.Rows(TableName::Probe).len(), 1);
	assert_eq!(Ctx.Rows(TableName::Probe)[0].Text("observed"), Some("high"));
}

#[test]
fn the_allowance_is_per_audience_so_another_holder_gets_its_own_observation() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.Now = 10;
	assert!(Probe(&mut Ctx, Probing()).Ok());

	Ctx.As("agent:b");
	Ctx.Now = 20;
	assert_eq!(
		Claim(
			&mut Ctx,
			ClaimArgs { Resource:"browser:second".to_string(), Task:"task:2".to_string(), ..Claiming() }
		),
		Answer::Accepted
	);
	let Answer = Probe(&mut Ctx, ProbeArgs { Resource:"browser:second".to_string(), ..Probing() });
	assert!(Answer.Ok());
	assert_eq!(
		Ctx.Rows(TableName::Probe)
			.iter()
			.map(|Row| Row.Text("audience").unwrap_or_default().to_string())
			.collect::<Vec<_>>(),
		vec!["agent:a", "agent:b"]
	);
}

#[test]
fn a_probe_carrying_a_stale_fence_is_refused_and_writes_nothing() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Ctx.Now = 1_001;
	assert_eq!(
		Claim(&mut Ctx, ClaimArgs { Task:"task:2".to_string(), ..Claiming() }),
		Answer::Accepted
	);
	let Mark = Ctx.Writes.len();
	assert_eq!(
		Probe(&mut Ctx, ProbeArgs { Fence:1, ..Probing() }),
		ProbeAnswer::Refused(Code::AdmissionRefused)
	);
	assert!(Written(&Ctx, Mark).is_empty());
	assert!(Ctx.Rows(TableName::Probe).is_empty());
}

#[test]
fn a_claim_or_a_probe_under_a_superseded_generation_is_refused_and_writes_nothing() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Accepted);
	Bound(&mut Ctx, Some("1"), "sha256:second");
	Ctx.Now = 10;
	let Mark = Ctx.Writes.len();
	assert_eq!(
		Claim(&mut Ctx, ClaimArgs { Resource:"browser:second".to_string(), ..Claiming() }),
		Answer::Refused(Code::AdmissionRefused)
	);
	assert_eq!(Probe(&mut Ctx, Probing()), ProbeAnswer::Refused(Code::AdmissionRefused));
	assert!(Written(&Ctx, Mark).is_empty());
	assert!(Ctx.Rows(TableName::Probe).is_empty());
}

#[test]
fn record_trace_appends_once_per_instant_kind_and_subject_and_refuses_different_content_under_the_same_key() {
	let mut Ctx = Catalog("agent:a", 1_000);
	Bound(&mut Ctx, None, "sha256:first");
	let Mark = Ctx.Writes.len();
	assert_eq!(RecordTrace(&mut Ctx, Tracing()), Answer::Accepted);
	assert_eq!(RecordTrace(&mut Ctx, Tracing()), Answer::Accepted);
	assert_eq!(Written(&Ctx, Mark).len(), 1);
	assert_eq!(
		Ctx.Rows(TableName::Trace)[0].Text("id"),
		Some(format!("1000:lease.claimed:{RESOURCE}").as_str())
	);
	assert_eq!(Ctx.Rows(TableName::Trace)[0].Text("sender"), Some("agent:a"));

	let Second = Ctx.Writes.len();
	assert_eq!(
		RecordTrace(&mut Ctx, Trace { Payload:"{\"fence\":2}".to_string(), ..Tracing() }),
		Answer::Refused(Code::AdmissionRefused)
	);
	assert!(Written(&Ctx, Second).is_empty());
}

#[test]
fn record_trace_refuses_a_generation_that_is_not_the_latest_bound() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	Bound(&mut Ctx, Some("1"), "sha256:second");
	let Mark = Ctx.Writes.len();
	assert_eq!(RecordTrace(&mut Ctx, Tracing()), Answer::Refused(Code::AdmissionRefused));
	assert!(Written(&Ctx, Mark).is_empty());
}

#[test]
fn every_timestamp_a_row_carries_is_the_contexts_now_so_a_fake_clock_decides_the_whole_transcript() {
	let mut Ctx = Catalog("agent:a", 1_234_567);
	let Mark = Ctx.Writes.len();
	Bound(&mut Ctx, None, "sha256:first");
	assert_eq!(Claim(&mut Ctx, ClaimArgs { Ttl:100, ..Claiming() }), Answer::Accepted);
	assert!(Probe(&mut Ctx, Probing()).Ok());
	assert_eq!(RecordTrace(&mut Ctx, Tracing()), Answer::Accepted);
	let mut Times:Vec<(&str, i64)> = Vec::new();
	for Write in Written(&Ctx, Mark) {
		let Some(Row) = &Write.Row else {
			continue;
		};
		for (Column, Value) in Row.Entries() {
			if (Column == "at" || Column.ends_with("At"))
				&& Column != "expiresAt"
				&& let Value::Number(Number) = Value
			{
				Times.push((Column, *Number));
			}
		}
	}
	assert!(Times.len() >= 4, "only {} timestamps were written", Times.len());
	for (Column, Value) in Times {
		assert_eq!(Value, 1_234_567, "{Column} did not come from the context's now");
	}
}

/// One malformed call, and the sentence that says what is wrong with it.
type Call = Box<dyn Fn(&mut Memory) -> bool>;

#[test]
fn every_rule_answers_a_malformed_argument_with_a_refusal_and_writes_nothing() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	let Mark = Ctx.Writes.len();
	let Calls:Vec<(&str, Call)> = vec![
		(
			"bind with no digest",
			Box::new(|Ctx:&mut Memory| {
				Bind(
					Ctx,
					BindArgs {
						Previous:None,
						Manifest:Manifest { Digest:String::new(), Capabilities:vec![BROWSE.to_string()] },
					},
				)
				.Ok()
			}),
		),
		(
			"bind naming one capability twice",
			Box::new(|Ctx:&mut Memory| {
				Bind(
					Ctx,
					BindArgs {
						Previous:None,
						Manifest:Manifest {
							Digest:"sha256:x".to_string(),
							Capabilities:vec![BROWSE.to_string(), BROWSE.to_string()],
						},
					},
				)
				.Ok()
			}),
		),
		(
			"claim with no resource",
			Box::new(|Ctx:&mut Memory| Claim(Ctx, ClaimArgs { Resource:String::new(), ..Claiming() }).Ok()),
		),
		(
			"claim under an unknown generation",
			Box::new(|Ctx:&mut Memory| Claim(Ctx, ClaimArgs { Generation:"nowhere".to_string(), ..Claiming() }).Ok()),
		),
		(
			"claim with no ttl",
			Box::new(|Ctx:&mut Memory| Claim(Ctx, ClaimArgs { Ttl:0, ..Claiming() }).Ok()),
		),
		(
			"renew with a negative ttl",
			Box::new(|Ctx:&mut Memory| Renew(Ctx, RenewArgs { Ttl:-1, ..Renewing() }).Ok()),
		),
		(
			"renew under an unknown generation",
			Box::new(|Ctx:&mut Memory| Renew(Ctx, RenewArgs { Generation:"nowhere".to_string(), ..Renewing() }).Ok()),
		),
		(
			"probe with no property",
			Box::new(|Ctx:&mut Memory| Probe(Ctx, ProbeArgs { Property:String::new(), ..Probing() }).Ok()),
		),
		(
			"probe at version zero",
			Box::new(|Ctx:&mut Memory| Probe(Ctx, ProbeArgs { Version:0, ..Probing() }).Ok()),
		),
		(
			"probe at a fence nobody holds",
			Box::new(|Ctx:&mut Memory| Probe(Ctx, ProbeArgs { Fence:99, ..Probing() }).Ok()),
		),
		(
			"trace with no kind",
			Box::new(|Ctx:&mut Memory| RecordTrace(Ctx, Trace { Kind:String::new(), ..Tracing() }).Ok()),
		),
		(
			"trace under an unknown generation",
			Box::new(|Ctx:&mut Memory| RecordTrace(Ctx, Trace { Generation:"nowhere".to_string(), ..Tracing() }).Ok()),
		),
	];
	for (Name, Call) in &Calls {
		assert!(!Call(&mut Ctx), "{Name} was accepted");
	}
	assert!(Written(&Ctx, Mark).is_empty());
}

/// The port's own addition: a ttl the clock cannot reach is a refusal, not a
/// wrap.
///
/// The source this was ported from added two numbers and let the language's
/// arithmetic answer; a signed integer that wraps would store an expiry in the
/// past and a lease that is born expired. Refusing is the same answer the
/// sibling runtime gives a lease whose ttl has no instant to expire at.
#[test]
fn a_claim_at_the_end_of_the_clock_is_refused_rather_than_wrapping() {
	let mut Ctx = Catalog("agent:a", i64::MAX);
	Bound(&mut Ctx, None, "sha256:first");
	let Mark = Ctx.Writes.len();
	assert_eq!(Claim(&mut Ctx, Claiming()), Answer::Refused(Code::AdmissionRefused));
	assert!(Written(&Ctx, Mark).is_empty());
}

/// The coherence predicate a view that lists live generations needs, on the
/// rows the rules wrote.
#[test]
fn latest_answers_the_successor_question_with_one_lookup() {
	let mut Ctx = Catalog("agent:a", 0);
	Bound(&mut Ctx, None, "sha256:first");
	assert!(Latest(&Ctx, "1").is_some());
	assert!(Latest(&Ctx, "9").is_none());
	Bound(&mut Ctx, Some("1"), "sha256:second");
	assert!(Latest(&Ctx, "1").is_none(), "a superseded generation is not the latest");
	assert!(Latest(&Ctx, "2").is_some());
}
