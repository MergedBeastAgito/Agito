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

use AgenticBrowserCDP::{Browse as BrowserBrowse, Error as BrowserError, OpenOptions, Target as BrowserTarget};
use AgenticChatDriver::{Failure, Page as ChatPage, Target};
use AgenticRuntime::{Call, FutureBox};
use serde_json::Value;

/// The chat driver's page, served by the browser substrate.
pub struct Page {
	Browse:Arc<dyn BrowserBrowse>,
	/// What each chat target was bound to, by URL.
	Held:Mutex<HashMap<String, BrowserTarget>>,
}

/// Bind the chat driver's `Page` to a browser.
pub fn Create(Browse:Arc<dyn BrowserBrowse>) -> Page { Page { Browse, Held:Mutex::new(HashMap::new()) } }

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

impl ChatPage for Page {
	fn Open<'a>(&'a self, Url:&'a str, Call:Call) -> FutureBox<'a, Result<Target, Failure>> {
		Box::pin(async move {
			let Target = self.Browse.Open(Url, OpenOptions::Launch(Call)).await.map_err(Classify)?;
			self.Lock().insert(Target.Url.clone(), Target.clone());
			Ok(Target { Url:Target.Url })
		})
	}

	fn Evaluate<'a>(
		&'a self,
		Target:&'a Target,
		Expression:&'a str,
		Call:Call,
	) -> FutureBox<'a, Result<Value, Failure>> {
		Box::pin(async move {
			let Held = self.Lock().get(&Target.Url).cloned();
			let Some(Held) = Held else {
				return Err(Failure::Of("this binding holds no browser target for the page"));
			};
			self.Browse.Evaluate(&Held, Expression, Call).await.map_err(Classify)
		})
	}

	fn Dispose<'a>(&'a self, Target:Target) -> FutureBox<'a, ()> {
		Box::pin(async move {
			let Held = self.Lock().remove(&Target.Url);
			if let Some(Held) = Held {
				self.Browse.Dispose(Held).await;
			}
		})
	}
}
