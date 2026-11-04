//! One condition plus the name it selects, and the ordered list of them.
//!
//! A rule list is the whole selection policy: first match wins, and the fallback
//! is a name rather than a rule, because "nothing else matched" is a different
//! statement from "this always holds" - a caller reading a trace should be able
//! to tell a lane chosen by its condition from one chosen by exhaustion.
//!
//! The same type serves both levels, because a lane and a template are selected
//! the same way: a name and a condition. A template rule naming a template the
//! selected lane does not serve is skipped rather than refused - a caller with
//! one rule list for every lane is the ordinary case, and a rule that cannot
//! apply to this lane is not a defect in it.

use crate::Struct::When::When;

/// One name and the condition that selects it.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
	/// The lane or template this rule selects.
	pub Name:String,
	/// The condition over the state.
	pub When:When,
}

impl Rule {
	/// A rule with a condition.
	pub fn New(Name:&str, When:When) -> Self { Self { Name:Name.to_string(), When } }

	/// A rule with no condition, which is a lane with no alternative.
	pub fn Always(Name:&str) -> Self { Self { Name:Name.to_string(), When:When::Always } }
}

/// An ordered list of rules and the name to fall back to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rules {
	/// The rules, in priority order.
	pub Ordered:Vec<Rule>,
	/// The name to select when no rule holds. `None` means nothing is selected
	/// and the caller is told so (`no-match`) rather than handed a lane nobody
	/// asked for.
	pub Default:Option<String>,
}

impl Rules {
	/// A list with no fallback.
	pub fn New(Ordered:Vec<Rule>) -> Self { Self { Ordered, Default:None } }

	/// The same list, with a fallback.
	pub fn Falling(mut self, Name:&str) -> Self {
		self.Default = Some(Name.to_string());
		self
	}

	/// Every name the list mentions, in order, including the fallback.
	pub fn Names(&self) -> Vec<&str> {
		self.Ordered
			.iter()
			.map(|Rule| Rule.Name.as_str())
			.chain(self.Default.as_deref())
			.collect()
	}

	/// Whether the list says nothing at all.
	pub fn IsEmpty(&self) -> bool { self.Ordered.is_empty() && self.Default.is_none() }
}
