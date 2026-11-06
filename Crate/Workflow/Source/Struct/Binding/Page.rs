//! The chat driver's `Page`, over the browser substrate.
//!
//! Three translations, and each one is a decision this layer owns:
//!
//! 1. **Ownership.** `Page` has no ownership because a chat driver must not
//!    care whether the browser was launched or attached. This binding launches,
//!    because a capability that asks a product a question owns the page it
//!    opened; a caller that wants a shared browser supplies a `Browse` that
//!    attaches instead.
//! 2. **The handle.** The driver passes back whatever `Open` returned and never
//!    reads it, so the browser target lives here, keyed by the URL the driver
//!    does keep. That is the whole of "a provider may hang whatever it needs
//!    there".
//! 3. **The vocabulary.** A browser failure has three codes and the chat port
//!    has four; the browser's code and message travel in the failure's
//!    `message`, and its observation in `detail`, so the driver can map the
//!    whole thing onto `not-ready` without losing which of the three happened.

use std::{
	collections::HashMap,
	sync::{Arc, Mutex, MutexGuard},
};

use ::CDP::{
	Browse as BrowserBrowse,
	Error as BrowserError,
	Input as BrowserInput,
	Modifiers as BrowserModifiers,
	OpenOptions,
	Press as BrowserPress,
	Target as BrowserTarget,
};
use ::Driver::{Failure, Input as ChatInput, Page as ChatPage, Target};
use ::RunTime::{Call, FutureBox};
use serde_json::Value;

/// The chat driver's page, served by the browser substrate.
pub struct Page {
	Browse:Arc<dyn BrowserBrowse>,
	/// Where to attach, when the caller wants a browser it did not start.
	Attach:Option<String>,
	/// Which page to bind on that browser, when it is not a new one.
	Bind:Option<String>,
	/// What each chat target was bound to, by URL.
	Held:Mutex<HashMap<String, BrowserTarget>>,
}

/// Bind the chat driver's `Page` to a browser this session launches.
pub fn Create(Browse:Arc<dyn BrowserBrowse>) -> Page {
	Page { Browse, Attach:None, Bind:None, Held:Mutex::new(HashMap::new()) }
}

/// Bind the chat driver's `Page` to a browser somebody else owns, and to the
/// page `Bind` names when it is given.
///
/// Attaching without a bind opens a page in that browser; attaching with one
/// drives the page that is already there, which is the difference between a
/// test run and a conversation with a tab a person is looking at.
pub fn Attached(Browse:Arc<dyn BrowserBrowse>, Endpoint:impl Into<String>, Bind:Option<String>) -> Page {
	Page { Browse, Attach:Some(Endpoint.into()), Bind, Held:Mutex::new(HashMap::new()) }
}

impl Page {
	fn Lock(&self) -> MutexGuard<'_, HashMap<String, BrowserTarget>> {
		self.Held.lock().unwrap_or_else(|Poison| Poison.into_inner())
	}
}

/// A browser failure, said in the chat port's vocabulary.
///
/// The code is kept: a consumer reading `detail` can still tell an unavailable
/// browser from a page that failed to load, which is the difference between
/// retrying and fixing the page.
fn Classify(Why:BrowserError) -> Failure {
	Failure {
		Message:format!("{}: {}", Why.Code().Name(), Why.Message()),
		Detail:Why.Detail().to_string(),
	}
}

/// The driver's input, said in the substrate's vocabulary.
///
/// A translation and not a cast: the two ports declare the same four modifiers
/// and the same two kinds of input because they are describing the same thing,
/// and each crate owning its own copy is what lets either one change without
/// the other. The cost of that is this function.
fn Translate(Input:&ChatInput) -> BrowserInput {
	match Input {
		ChatInput::Press(Chord) => {
			BrowserInput::Press(BrowserPress {
				Key:Chord.Key.clone(),
				Modifiers:BrowserModifiers {
					Alt:Chord.Modifiers.Alt,
					Control:Chord.Modifiers.Control,
					Meta:Chord.Modifiers.Meta,
					Shift:Chord.Modifiers.Shift,
				},
				Command:Chord.Command.clone(),
			})
		},
		ChatInput::Type(Text) => BrowserInput::Type(Text.clone()),
	}
}

impl ChatPage for Page {
	fn Open<'a>(&'a self, URL:&'a str, Call:Call) -> FutureBox<'a, Result<Target, Failure>> {
		Box::pin(async move {
			let Options = match (&self.Attach, &self.Bind) {
				(Some(Endpoint), Some(Key)) => OpenOptions::Existing(Endpoint.clone(), Key.clone(), Call),
				(Some(Endpoint), None) => OpenOptions::Attach(Endpoint.clone(), Call),
				(None, _) => OpenOptions::Launch(Call),
			};
			let Target = self.Browse.Open(URL, Options).await.map_err(Classify)?;
			self.Lock().insert(Target.URL.clone(), Target.clone());
			Ok(Target { URL:Target.URL })
		})
	}

	fn Evaluate<'a>(
		&'a self,
		Target:&'a Target,
		Expression:&'a str,
		Call:Call,
	) -> FutureBox<'a, Result<Value, Failure>> {
		Box::pin(async move {
			let Held = self.Lock().get(&Target.URL).cloned();
			let Some(Held) = Held else {
				return Err(Failure::Of("this binding holds no browser target for the page"));
			};
			self.Browse.Evaluate(&Held, Expression, Call).await.map_err(Classify)
		})
	}

	fn Input<'a>(&'a self, Target:&'a Target, Input:&'a ChatInput, Call:Call) -> FutureBox<'a, Result<(), Failure>> {
		Box::pin(async move {
			let Held = self.Lock().get(&Target.URL).cloned();
			let Some(Held) = Held else {
				return Err(Failure::Of("this binding holds no browser target for the page"));
			};
			let Translated = Translate(Input);
			self.Browse.Input(&Held, &Translated, Call).await.map_err(Classify)
		})
	}

	fn Dispose<'a>(&'a self, Target:Target) -> FutureBox<'a, ()> {
		Box::pin(async move {
			let Held = self.Lock().remove(&Target.URL);
			if let Some(Held) = Held {
				self.Browse.Dispose(Held).await;
			}
		})
	}
}
