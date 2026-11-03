//! The concrete substrate: `chromiumoxide`, and the only file that names it.
//!
//! Two entry points, because attach and launch are different operations: `Launch`
//! starts a browser process this session owns, and `Attach` connects to one that is
//! already listening. The session decides which to call from
//! [`crate::Struct::Port::OpenOptions::AttachTo`], and records the ownership it
//! implies, so disposal closes only what was launched.
//!
//! The handler task is not optional. `chromiumoxide`'s usage model is a `Browser`
//! plus a `Handler` that must be polled: a client that keeps the browser and drops
//! the handler gets a connection that accepts commands and never answers them. The
//! task is owned here and aborted on release.
//!
//! A caller must already be inside a tokio runtime: `Launch` and `Attach` spawn the
//! handler, and a spawn outside a runtime panics. That is the composition root's
//! contract, and the only place in the workspace where it applies.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use chromiumoxide::{Browser, BrowserConfig, Handler, Page};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::task::JoinHandle;

use AgenticRuntime::{Call, FutureBox};

use crate::Fn::Classify::{Classify, Fatal};
use crate::Struct::Error::Error;
use crate::Struct::Transport::{Launcher, Transport};

/// How to start or reach a browser.
#[derive(Clone, Debug, Default)]
pub struct ChromeOptions {
	/// The browser binary, when it is not the one `chromiumoxide` finds.
	pub Executable: Option<PathBuf>,
	/// Whether to run without a window. Off means a visible browser, which is what
	/// an attached session usually wants to share with a person.
	pub Headless: bool,
	/// Whether to pass `--no-sandbox`. Needed in containers, never on a desktop.
	pub NoSandbox: bool,
}

/// The `chromiumoxide` launcher.
pub struct Chrome {
	Options: ChromeOptions,
}

impl Chrome {
	/// A launcher with the caller's configuration.
	pub fn New(Options: ChromeOptions) -> Arc<Self> {
		Arc::new(Self { Options })
	}

	/// A browser whose handler is already being polled, bound to `Url`.
	async fn Bound(
		Browser: Browser,
		Handler: Handler,
		Url: &str,
	) -> Result<Arc<dyn Transport>, Error> {
		let Endpoint = Browser.websocket_address().clone();
		let Task = tokio::spawn(async move {
			let mut Handler = Handler;
			while Handler.next().await.is_some() {}
		});
		let Page = Browser
			.new_page(Url)
			.await
			.map_err(|Why| Classify(&Why))?;
		Ok(Arc::new(ChromeTransport {
			Page,
			Browser: Mutex::new(Some(Browser)),
			Handler: Mutex::new(Some(Task)),
			Endpoint,
			Closed: AtomicBool::new(false),
		}))
	}
}

impl Launcher for Chrome {
	fn Launch<'a>(
		&'a self,
		Url: &'a str,
		_Call: Call,
	) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>> {
		Box::pin(async move {
			let mut Builder = BrowserConfig::builder();
			if let Some(Path) = &self.Options.Executable {
				Builder = Builder.chrome_executable(Path);
			}
			if self.Options.Headless {
				Builder = Builder.new_headless_mode();
			} else {
				Builder = Builder.with_head();
			}
			if self.Options.NoSandbox {
				Builder = Builder.no_sandbox();
			}
			let Config = Builder.build().map_err(|Why| {
				Error::Unavailable("the browser configuration is not usable", Why.to_string())
			})?;
			let (Browser, Handler) = Browser::launch(Config).await.map_err(|Why| Classify(&Why))?;
			Self::Bound(Browser, Handler, Url).await
		})
	}

	fn Attach<'a>(
		&'a self,
		Endpoint: &'a str,
		Url: &'a str,
		_Call: Call,
	) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>> {
		Box::pin(async move {
			let (Browser, Handler) = Browser::connect(Endpoint)
				.await
				.map_err(|Why| Classify(&Why))?;
			Self::Bound(Browser, Handler, Url).await
		})
	}
}

/// One page on one browser, and the handler that keeps it answering.
struct ChromeTransport {
	Page: Page,
	Browser: Mutex<Option<Browser>>,
	Handler: Mutex<Option<JoinHandle<()>>>,
	Endpoint: String,
	Closed: AtomicBool,
}

impl ChromeTransport {
	fn Lock<T>(Guard: &Mutex<T>) -> MutexGuard<'_, T> {
		Guard.lock().unwrap_or_else(|Poison| Poison.into_inner())
	}

	/// Stop polling the handler, which is what ends this session's side of the
	/// connection whatever happens to the browser.
	fn Stop(&self) {
		if let Some(Handler) = Self::Lock(&self.Handler).take() {
			Handler.abort();
		}
		self.Closed.store(true, Ordering::SeqCst);
	}
}

impl Transport for ChromeTransport {
	fn Endpoint(&self) -> &str {
		&self.Endpoint
	}

	fn Closed(&self) -> bool {
		self.Closed.load(Ordering::SeqCst)
	}

	fn Evaluate<'a>(
		&'a self,
		Expression: &'a str,
		_Call: Call,
	) -> FutureBox<'a, Result<Value, Error>> {
		Box::pin(async move {
			match self.Page.evaluate(Expression).await {
				Ok(Result) => Ok(Result.value().cloned().unwrap_or(Value::Null)),
				Err(Why) => {
					if Fatal(&Why) {
						self.Closed.store(true, Ordering::SeqCst);
					}
					Err(Classify(&Why))
				}
			}
		})
	}

	fn Close(&self) -> FutureBox<'_, ()> {
		Box::pin(async move {
			// Closing the page first runs its unload hooks; then the browser this
			// session launched, which nobody else may close.
			let _ = self.Page.clone().close().await;
			// Taken out of the lock before the await: a guard held across one is not
			// `Send`, and a browser handle is the only thing that needs to survive.
			let Browser = Self::Lock(&self.Browser).take();
			if let Some(mut Browser) = Browser {
				let _ = Browser.close().await;
			}
			self.Stop();
		})
	}

	fn Detach(&self) -> FutureBox<'_, ()> {
		Box::pin(async move {
			// Somebody else owns this browser: stop listening and leave it running.
			self.Stop();
		})
	}
}
