//! What `agentic` can be asked to do.

use clap::{Parser, Subcommand};

/// The composition root.
#[derive(Clone, Debug, Parser)]
#[command(
	name = "agentic",
	about = "Compose the agentic capabilities: skills, design verification, chat and the browser they share.",
	version
)]
pub struct Command {
	/// What to do.
	#[command(subcommand)]
	pub Verb:Verb,
}

/// One verb.
#[derive(Clone, Debug, Subcommand)]
pub enum Verb {
	/// List the modules this workspace ships, with the port each provides.
	Manifests,
	/// Report one module's readiness, and whether its manifest still matches
	/// its code.
	Verify {
		/// The module to report, by its manifest name.
		#[arg(long)]
		Module:String,
	},
	/// Discover, validate and digest a skill corpus. Needs no browser.
	Skills {
		/// The corpus root: a directory of skill directories.
		Root:String,
		/// Print the whole record as JSON instead of a summary.
		#[arg(long)]
		Json:bool,
	},
	/// Open a page and evaluate one expression in it.
	Browse {
		/// The page to open. With `--bind` it is only a fallback.
		#[arg(long)]
		Url:String,
		/// The expression to evaluate.
		#[arg(long, default_value = "1 + 1")]
		Expression:String,
		/// Attach to a browser already listening here instead of launching one.
		#[arg(long)]
		Attach:Option<String>,
		/// Bind the page the attached browser already has, by target id or URL
		/// prefix, instead of opening a new one.
		#[arg(long)]
		Bind:Option<String>,
		/// The whole operation's budget, in milliseconds.
		#[arg(long, default_value_t = 30_000)]
		Timeout:u64,
	},
	/// Ask a chat product one question and print the answer once it settles.
	Chat {
		/// The chat product's URL. With `--bind` it is only a fallback.
		#[arg(long)]
		Url:String,
		/// The model the answer must come from, as the page labels it.
		#[arg(long)]
		Model:String,
		/// The question, sent as written.
		#[arg(long)]
		Question:String,
		/// Attach to a browser already listening here instead of launching one.
		#[arg(long)]
		Attach:Option<String>,
		/// Bind the page the attached browser already has, by target id or URL
		/// prefix, instead of opening a new one.
		#[arg(long)]
		Bind:Option<String>,
		/// A JSON file of selector overrides, merged over the defaults.
		#[arg(long)]
		Selectors:Option<String>,
		/// How the answer is read: `markdown` keeps the page's tables, fences
		/// and TeX, `text` is the rendered text as it always was. Absent, the
		/// selector file decides, and its default is `text`.
		#[arg(long)]
		Format:Option<String>,
		/// Report every read of the thread as a JSON line on stderr while the
		/// answer is being produced, instead of only the settled answer.
		#[arg(long)]
		Observe:bool,
		/// The whole call's budget, in milliseconds.
		#[arg(long, default_value_t = 120_000)]
		Timeout:u64,
	},
	/// Measure a page's named elements and print what was read.
	Design {
		/// The page to measure.
		#[arg(long)]
		Url:String,
		/// A selector to measure. Repeat for each one.
		#[arg(long = "selector", required = true)]
		Selectors:Vec<String>,
		/// The whole measurement's budget, in milliseconds.
		#[arg(long, default_value_t = 30_000)]
		Timeout:u64,
	},
}

impl Command {
	/// Read the process's arguments.
	pub fn Parse() -> Self { <Self as Parser>::parse() }

	/// Read a specific argument list, for a test that must not touch the
	/// process.
	pub fn ParseFrom(Arguments:Vec<String>) -> Result<Self, clap::Error> { <Self as Parser>::try_parse_from(Arguments) }
}
