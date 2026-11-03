//! The mapping from `chromiumoxide`'s errors onto the three codes.
//!
//! The rule is the port's, applied to a substrate that has more failure modes than
//! the port has codes:
//!
//! - **a dead socket is `browser-unavailable`.** A WebSocket error, an I/O error,
//!   no response at all, a channel that closed under a request, and a browser
//!   process that exited or never announced its endpoint all mean the same thing to
//!   a consumer: there is no browser to talk to. So do the failures that can only
//!   happen before a page exists - a URL that does not parse, a response that does
//!   not decode, a frame id that is not there.
//! - **a page that could not be used is `navigation-failed`.** A JavaScript
//!   exception, a protocol error naming the page, a scrolling failure: the transport
//!   is alive and the page is what went wrong.
//! - **a deadline is `timeout`.** `CdpError::Timeout` is the protocol's own timeout,
//!   which is the same event as the session's deadline from a consumer's side.
//!
//! `Fatal` is the second half of the mapping: it says which errors mean the
//! transport is gone, so the transport can report itself closed and the session can
//! classify the *next* failure as `browser-unavailable` rather than as a page
//! defect.

use chromiumoxide::error::CdpError;

use crate::Struct::Error::Error;

/// Map one substrate error onto the port's three codes, keeping the provider's
/// words in `detail`.
pub fn Classify(Cdp: &CdpError) -> Error {
	match Cdp {
		CdpError::Ws(Why) => Error::Unavailable(
			"the browser could not be reached",
			format!("the websocket failed: {Why}"),
		),
		CdpError::Io(Why) => Error::Unavailable(
			"the browser could not be reached",
			format!("the connection failed: {Why}"),
		),
		CdpError::NoResponse => Error::Unavailable(
			"the browser did not answer",
			"the request produced no response",
		),
		CdpError::ChannelSendError(Why) => Error::Unavailable(
			"the browser is no longer listening",
			format!("the request channel closed: {Why}"),
		),
		CdpError::LaunchExit(Status, _) => Error::Unavailable(
			"the browser exited before it was ready",
			format!("the process exited with {Status:?}"),
		),
		CdpError::LaunchTimeout(_) => Error::Unavailable(
			"the browser never announced its endpoint",
			"the launch timed out",
		),
		CdpError::LaunchIo(Why, _) => Error::Unavailable(
			"the browser could not be started",
			format!("the process could not be spawned: {Why}"),
		),
		CdpError::NotFound => Error::Unavailable(
			"the browser did not answer",
			"the requested value was not found",
		),
		CdpError::Url(Why) => Error::Unavailable(
			"the browser could not be reached",
			format!("the endpoint is not a URL: {Why}"),
		),
		CdpError::Serde(Why) => Error::Unavailable(
			"the browser answered something this client cannot read",
			format!("the response did not decode: {Why}"),
		),
		CdpError::DecodeError(Why) => Error::Unavailable(
			"the browser answered something this client cannot read",
			format!("the payload did not decode: {Why}"),
		),
		CdpError::FrameNotFound(Frame) => Error::Unavailable(
			"the page is no longer there",
			format!("frame {Frame:?} was not found"),
		),
		CdpError::Timeout => Error::Timeout(
			"the page did not answer in time",
			"the protocol reported a timeout",
		),
		CdpError::JavascriptException(Details) => Error::Navigation(
			"the page threw while it was being asked",
			format!("{Details:?}"),
		),
		CdpError::ChromeMessage(Message) => {
			Error::Navigation("the page could not answer", Message.clone())
		}
		CdpError::Chrome(Why) => Error::Navigation(
			"the page could not answer",
			format!("the protocol reported {Why}"),
		),
		CdpError::ScrollingFailed(Why) => Error::Navigation(
			"the page could not be scrolled",
			Why.clone(),
		),
		CdpError::UnexpectedWsMessage(Message) => Error::Navigation(
			"the browser sent a message this client does not handle",
			format!("{Message:?}"),
		),
		CdpError::InvalidMessage(Message, Why) => Error::Unavailable(
			"the browser sent a message this client cannot read",
			format!("{Message}: {Why}"),
		),
	}
}

/// Whether the transport is gone after this error.
pub fn Fatal(Cdp: &CdpError) -> bool {
	matches!(Classify(Cdp).Code(), crate::Struct::Error::Code::Unavailable)
}

/// The page-error rule: a closed transport wins over a page defect, because the
/// socket is gone rather than the page being wrong. A deadline is left alone - it
/// is the caller's own bound, not a fact about the socket.
pub fn ClassifyPage(Why: Error, Closed: bool) -> Error {
	match Why.Code() {
		crate::Struct::Error::Code::Timeout => Why,
		_ if Closed => Error::Unavailable(
			"the transport closed while the page was being asked",
			Why.Detail().to_string(),
		),
		_ => Why,
	}
}
