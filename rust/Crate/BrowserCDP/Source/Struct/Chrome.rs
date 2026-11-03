//! The concrete substrate: `chromiumoxide`, and the only file that names it.
//!
//! Two entry points, because attach and launch are different operations:
//! `Launch` starts a browser process this session owns, and `Attach` connects
//! to one that is already listening. The session decides which to call from
//! [`crate::Struct::Port::OpenOptions::AttachTo`], and records the ownership it
//! implies, so disposal closes only what was launched.
//!
//! The handler task is not optional. `chromiumoxide`'s usage model is a
//! `Browser` plus a `Handler` that must be polled: a client that keeps the
//! browser and drops the handler gets a connection that accepts commands and
//! never answers them. The task is owned here and aborted on release.
//!
//! A caller must already be inside a tokio runtime: `Launch` and `Attach` spawn
//! the handler, and a spawn outside a runtime panics. That is the composition
//! root's contract, and the only place in the workspace where it applies.

use std::{
	path::PathBuf,
	sync::{
		Arc,
		Mutex,
		MutexGuard,
		atomic::{AtomicBool, Ordering},
	},
};

use AgenticRuntime::{Call, FutureBox};
use chromiumoxide::{Browser, BrowserConfig, Handler, Page};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::task::JoinHandle;

use crate::{
	Fn::Classify::{Classify, Fatal},
	Struct::{
		Error::Error,
		Transport::{Launcher, PageWant, Transport},
	},
};

/// How to start or reach a browser.
#[derive(Clone, Debug, Default)]
pub struct ChromeOptions {
	/// The browser binary, when it is not the one `chromiumoxide` finds.
	pub Executable:Option<PathBuf>,
	/// Whether to run without a window. Off means a visible browser, which is
	/// what an attached session usually wants to share with a person.
	pub Headless:bool,
	/// Whether to pass `--no-sandbox`. Needed in containers, never on a
	/// desktop.
	pub NoSandbox:bool,
}

/// The `chromiumoxide` launcher.
pub struct Chrome {
	Options:ChromeOptions,
}

impl Chrome {
	/// A launcher with the caller's configuration.
	pub fn New(Options:ChromeOptions) -> Arc<Self> { Arc::new(Self { Options }) }

	/// How long an attach waits for the browser to hand over the sessions of
	/// pages that were already open.
	///
	/// Attaching to a target is a round trip, so a page is not in the handler's
	/// map the instant it is requested. This is that moment, not a timeout on
	/// the browser: it is bounded so a browser that really does not hold the
	/// page is reported rather than waited on.
	pub const PageGrace:std::time::Duration = std::time::Duration::from_secs(2);

	/// The page an attach binds to, chosen by target id first and by URL
	/// prefix second.
	///
	/// Target id is exact and URL is not, so the order matters: two tabs of one
	/// product are told apart by id, and a caller that only knows the URL gets
	/// the first page that starts with it. A key that matches nothing is
	/// `navigation-failed` - the browser answered, and the page the caller
	/// named is not in it.
	async fn Bound(Key:&str, Pages:Vec<Page>) -> Result<Page, Error> {
		let mut Fallback = None;
		let mut Seen = Vec::new();
		for Page in Pages {
			let Id = Page.target_id().as_ref().to_string();
			let Url = Page.url().await.ok().flatten().unwrap_or_default();
			Seen.push(format!("{Id} {Url}"));
			if Id == Key {
				return Ok(Page);
			}
			if Fallback.is_none() && !Url.is_empty() && Url.starts_with(Key) {
				Fallback = Some(Page);
			}
		}
		match Fallback {
			Some(Page) => Ok(Page),
			None => Err(Error::Navigation(
				"the browser holds no page matching the one asked for",
				format!("no target id or URL starts with {Key}; it holds [{}]", Seen.join(" | ")),
			)),
		}
	}

	/// A browser whose handler is already being polled, bound to `Want`.
	async fn Bind(mut Browser:Browser, Handler:Handler, Want:PageWant<'_>) -> Result<Arc<dyn Transport>, Error> {
		let Endpoint = Browser.websocket_address().clone();
		let Task = tokio::spawn(async move {
			let mut Handler = Handler;
			while Handler.next().await.is_some() {}
		});
		let Page = match Want {
			PageWant::Fresh(Url) => Browser.new_page(Url).await.map_err(|Why| Classify(&Why))?,
			PageWant::Existing(Key) => {
				// A browser announces no `targetCreated` for the tabs that were
				// already open when this client connected, so the handler's own
				// map is empty until it asks. Without this fetch every bind
				// reports that the page is not there.
				if let Err(Why) = Browser.fetch_targets().await {
					return Err(Error::Unavailable(
						"the browser would not list its pages",
						format!("target enumeration failed: {Why}"),
					));
				}
				// The attach that `fetch_targets` starts is a round trip, and a
				// page cannot be handed out until its session arrives. So the
				// match is retried until the grace runs out, and a browser that
				// holds other pages but not this one is still reported as not
				// holding it - which is the difference between a race and a
				// wrong target id.
				let Start = std::time::Instant::now();
				loop {
					let Pages = Browser.pages().await.map_err(|Why| Classify(&Why))?;
					match Self::Bound(Key, Pages).await {
						Ok(Page) => break Page,
						Err(_) if Start.elapsed() < Self::PageGrace => {
							tokio::time::sleep(std::time::Duration::from_millis(25)).await;
						},
						Err(Why) => return Err(Why),
					}
				}
			},
		};
		let Url = Page.url().await.ok().flatten().unwrap_or_default();
		Ok(Arc::new(ChromeTransport {
			Page,
			Url,
			Browser:Mutex::new(Some(Browser)),
			Handler:Mutex::new(Some(Task)),
			Endpoint,
			Closed:AtomicBool::new(false),
		}))
	}
}

impl Launcher for Chrome {
	fn Launch<'a>(&'a self, Url:&'a str, _Call:Call) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>> {
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
			let Config = Builder
				.build()
				.map_err(|Why| Error::Unavailable("the browser configuration is not usable", Why.to_string()))?;
			let (Browser, Handler) = Browser::launch(Config).await.map_err(|Why| Classify(&Why))?;
			Self::Bind(Browser, Handler, PageWant::Fresh(Url)).await
		})
	}

	fn Attach<'a>(
		&'a self,
		Endpoint:&'a str,
		Want:PageWant<'a>,
		_Call:Call,
	) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>> {
		Box::pin(async move {
			let (Browser, Handler) = Browser::connect(Endpoint).await.map_err(|Why| Classify(&Why))?;
			Self::Bind(Browser, Handler, Want).await
		})
	}
}

/// One page on one browser, and the handler that keeps it answering.
struct ChromeTransport {
	Page:Page,
	/// Where the page was when this transport was bound.
	Url:String,
	Browser:Mutex<Option<Browser>>,
	Handler:Mutex<Option<JoinHandle<()>>>,
	Endpoint:String,
	Closed:AtomicBool,
}

impl ChromeTransport {
	fn Lock<T>(Guard:&Mutex<T>) -> MutexGuard<'_, T> { Guard.lock().unwrap_or_else(|Poison| Poison.into_inner()) }

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
	fn Endpoint(&self) -> &str { &self.Endpoint }

	fn Url(&self) -> &str { &self.Url }

	fn Closed(&self) -> bool { self.Closed.load(Ordering::SeqCst) }

	fn Evaluate<'a>(&'a self, Expression:&'a str, _Call:Call) -> FutureBox<'a, Result<Value, Error>> {
		Box::pin(async move {
			match self.Page.evaluate(Expression).await {
				Ok(Result) => Ok(Result.value().cloned().unwrap_or(Value::Null)),
				Err(Why) => {
					if Fatal(&Why) {
						self.Closed.store(true, Ordering::SeqCst);
					}
					Err(Classify(&Why))
				},
			}
		})
	}

	fn Close(&self) -> FutureBox<'_, ()> {
		Box::pin(async move {
			// Closing the page first runs its unload hooks; then the browser
			// this session launched, which nobody else may close.
			let _ = self.Page.clone().close().await;
			// Taken out of the lock before the await: a guard held across one
			// is not `Send`, and a browser handle is the only thing
			// that needs to survive.
			let Browser = Self::Lock(&self.Browser).take();
			if let Some(mut Browser) = Browser {
				let _ = Browser.close().await;
			}
			self.Stop();
		})
	}

	fn Detach(&self) -> FutureBox<'_, ()> {
		Box::pin(async move {
			// Somebody else owns this browser: stop listening and leave it
			// running.
			self.Stop();
		})
	}
}
