//! Completion as an explicit state, not a loop with a counter.
//!
//! Three consecutive reads must report no change before an answer counts as
//! settled, and the counter counts *comparisons*: the third read that equals the
//! one before it is what settles the answer, so a page that delivers its last
//! token and stops is read four times in all. Fewer than three is a page that
//! paused mid-sentence; more than three is latency nobody asked for.
//!
//! Written as a state machine because a loop with a counter cannot say what it is
//! doing: `Reading { Text, Confirmed }` is a claim about the page - "this text has
//! been confirmed twice" - and `Settled(Text)` is the only state a caller may read
//! an answer out of. The empty state is not an answer either: a thread with no
//! answer yet resets the machine rather than confirming an empty string.

/// How many comparisons must agree before an answer counts as settled.
pub const StableReads: u8 = 3;

/// How many consecutive reads have confirmed the text currently held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Confirmations(u8);

impl Confirmations {
	/// No confirmation yet: the text was just read for the first time.
	pub const fn First() -> Self {
		Self(0)
	}

	/// One more read that agreed.
	pub const fn Next(self) -> Self {
		Self(self.0.saturating_add(1))
	}

	/// How many comparisons have agreed.
	pub const fn Count(self) -> u8 {
		self.0
	}

	/// Whether this many confirmations is enough to settle an answer.
	pub const fn Settles(self) -> bool {
		self.0 >= StableReads
	}
}

/// What the driver knows about the answer it is reading.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Settlement {
	/// Nothing has been read yet, or the newest answer is still empty.
	#[default]
	Empty,
	/// Text the page has shown, and how many reads have agreed on it.
	Reading {
		/// The text as last read.
		Text: String,
		/// How many consecutive reads have confirmed it.
		Confirmed: Confirmations,
	},
	/// Text three reads agreed on. This is the only state an answer comes from.
	Settled(String),
}

impl Settlement {
	/// Fold one read into the state, and say whether it is settled.
	///
	/// `Current` is the answer's text when this question caused a new answer, and
	/// `None` when the thread has not grown: an unchanged thread is not a
	/// confirmation of anything, so it resets the state instead of advancing it.
	pub fn Observe(&mut self, Current: Option<&str>) -> bool {
		let Current = match Current {
			Some(Text) if !Text.is_empty() => Text,
			_ => {
				*self = Settlement::Empty;
				return false;
			}
		};
		*self = match std::mem::take(self) {
			Settlement::Reading { Text, Confirmed } if Text == Current => {
				let Confirmed = Confirmed.Next();
				if Confirmed.Settles() {
					Settlement::Settled(Current.to_string())
				} else {
					Settlement::Reading {
						Text,
						Confirmed,
					}
				}
			}
			_ => Settlement::Reading {
				Text: Current.to_string(),
				Confirmed: Confirmations::First(),
			},
		};
		matches!(self, Settlement::Settled(_))
	}

	/// The text this state holds, if it holds one.
	pub fn Text(&self) -> Option<&str> {
		match self {
			Settlement::Empty => None,
			Settlement::Reading { Text, .. } => Some(Text),
			Settlement::Settled(Text) => Some(Text),
		}
	}
}
