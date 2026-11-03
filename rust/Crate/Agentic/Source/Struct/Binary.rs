//! The command line, as data.
//!
//! One verb per thing the composition root can actually do, and no verb for
//! anything a capability does on its own: `skills` and `verify` need no
//! browser, and `browse`, `chat` and `design` need one. That split is the
//! release story - a machine with no browser can still verify the corpus and
//! every manifest.

pub mod Command;
