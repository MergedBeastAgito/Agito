//! One composition that uses four capabilities in the order a release check
//! does.
//!
//! A corpus is swept, every skill is validated, the corpus is digested, and
//! then a built page is measured and judged. It is written here rather than in
//! a capability because it is the composition that knows all four, and it is
//! written at all because a workflow crate with no workflow in it is a
//! directory.
//!
//! The order is the interesting part: the corpus is read before the page is
//! touched, so a release check that finds a broken skill does not pay for a
//! browser, and a page that cannot be measured does not hide the findings that
//! were already collected.

use ::Verification::{Invariant, Measurement, Verdict, Verify};
use ::RunTime::{Call, FutureBox};
use ::Contract::{Finding, Skill, SkillContract};

/// What one release check found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
	/// Every skill the corpus holds.
	pub Skills:Vec<Skill>,
	/// Every rule a skill broke.
	pub Findings:Vec<Finding>,
	/// The corpus digest, which is stable across checkouts.
	pub Digest:String,
	/// What the page measured, when there was a page to measure.
	pub Measurements:Vec<Measurement>,
	/// The verdict, when there was one.
	pub Verdict:Option<Verdict>,
	/// Why the page could not be measured, when it could not be.
	pub Failure:Option<String>,
}

impl Outcome {
	/// Whether the corpus is clean and the page held.
	pub fn Held(&self) -> bool {
		self.Findings.is_empty() && self.Failure.is_none() && self.Verdict.as_ref().is_none_or(|Verdict| Verdict.Held)
	}
}

/// Sweep a corpus, then measure and judge a page.
pub fn Verify<'a>(
	Contract:&'a dyn SkillContract,
	Verify:&'a dyn Verify,
	Root:&'a str,
	Selectors:&'a [String],
	Invariants:&'a [Invariant],
	Call:Call,
) -> FutureBox<'a, Outcome> {
	Box::pin(async move {
		let Skills = Contract.Discover(Root).await;
		let mut Findings = Vec::new();
		for Skill in &Skills {
			Findings.extend(Contract.Validate(Skill).await);
		}
		let Digest = Contract.Digest(&Skills);
		let mut Outcome = Outcome { Skills, Findings, Digest, ..Outcome::default() };

		if Selectors.is_empty() {
			return Outcome;
		}

		match Verify.Measure(Selectors, Call).await {
			Ok(Measurements) => {
				Outcome.Verdict = Some(Verify.Check(Invariants, &Measurements));
				Outcome.Measurements = Measurements;
			},
			Err(Why) => Outcome.Failure = Some(format!("{}: {}", Why.Code().Name(), Why.Message())),
		}
		Outcome
	})
}
