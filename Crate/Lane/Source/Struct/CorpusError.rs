//! What can go wrong while a corpus is being read, and the path or id that says
//! which part of it.
//!
//! This is the *load-time* error, and it is deliberately not
//! [`crate::Struct::Error::Error`]: the port's error has seven codes fixed by
//! `module.json` and is `Clone + Eq` because it travels in a trace, while a load
//! failure carries an `io::Error` or a `serde_json::Error` and can be neither.
//! Keeping them apart is what lets the loader say exactly what it found - the
//! path, the lane, the template id - instead of collapsing every refusal into
//! one sentence, and a caller that only knows the port still reaches it with `?`
//! through the `From` impl at the bottom of this file.
//!
//! # Every variant names what it is about
//!
//! A failed load is read by a person holding a directory, not by a program
//! holding a code, so [`CorpusError::Message`] carries the path or the id that
//! was wrong. A refusal that says "the corpus is invalid" is a refusal that
//! sends its reader to open three files and guess.
//!
//! # Message and detail, the split the port already makes
//!
//! [`CorpusError::Message`] is the sentence; [`CorpusError::Detail`] is the
//! evidence - the parser's line and column, the files that were searched, the id
//! that was not found. Both are methods rather than fields so the variants can
//! stay typed: a caller matches on `UnknownTemplateRef` to learn the lane, and
//! prints the message when it only wants to say what happened.

use std::{
	fmt,
	io,
	path::{Path, PathBuf},
};

use crate::Struct::Error::{Code, Error};

/// Every way reading a corpus can fail.
#[derive(Debug)]
pub enum CorpusError {
	/// A file the layout requires is not there.
	MissingFile {
		/// The path that was expected and is absent.
		Path:PathBuf,
	},
	/// A file is there and could not be read.
	Io {
		/// The path that was read.
		Path:PathBuf,
		/// What the operating system said.
		Source:io::Error,
	},
	/// A file is there, was read, and is not JSON.
	MalformedJSON {
		/// The path that was read.
		Path:PathBuf,
		/// Which file of the layout it is, in the corpus's own words ("the
		/// observation file"), so the sentence says what the reader was
		/// expecting to find there rather than only where the bytes came from.
		Role:&'static str,
		/// What the parser said, with its line and column.
		Source:serde_json::Error,
	},
	/// A file is valid JSON but not the shape the corpus layout puts there.
	///
	/// Separate from [`CorpusError::MalformedJSON`] because the two are read
	/// differently: a file that is not JSON is a file to open, while a file that
	/// holds an array where the layout wants an object is a corpus that was
	/// written to a different schema.
	UnexpectedShape {
		/// The path the shape was found in.
		Path:PathBuf,
		/// Which part of the file, in the corpus's own words.
		What:String,
		/// What was actually there.
		Found:String,
	},
	/// No file in the corpus declares a lane.
	NoLanes {
		/// Every file the loader searched, in the order it read them, so the
		/// reader can see where it looked rather than guess.
		Searched:Vec<PathBuf>,
	},
	/// Two lane sources declare the same lane.
	///
	/// Refused rather than merged: the second declaration would silently replace
	/// the first, and which one wins would depend on a file name.
	DuplicateLane {
		/// The lane declared twice.
		Lane:String,
		/// The file that declared it first.
		First:PathBuf,
		/// The file that declared it again.
		Second:PathBuf,
	},
	/// A lane names a template no template file declares.
	UnknownTemplateRef {
		/// The lane holding the reference.
		Lane:String,
		/// The template id the lane names.
		TemplateID:String,
		/// Every template the corpus does declare, so the refusal says what
		/// the lane could have named instead of only what it got wrong.
		Available:Vec<String>,
	},
	/// A lane sends a field the observation neither reads nor derives.
	UnknownFieldRef {
		/// The lane sending the field.
		Lane:String,
		/// The field the lane names.
		Field:String,
		/// Every field the observation does carry (read or derived), so the
		/// reader sees the vocabulary the lane was written against.
		Available:Vec<String>,
	},
	/// A lane states a condition the corpus's vocabulary cannot express.
	///
	/// The condition is read by [`crate::Struct::Predicate::Predicate`]'s own
	/// schema, which owns the operator vocabulary; this variant is the lane it
	/// was written on plus that reader's own complaint, so a corpus author is
	/// told which lane to open and why its condition was refused.
	BadCondition {
		/// The lane whose condition could not be read.
		Lane:String,
		/// The reader's sentence.
		Message:String,
		/// The reader's evidence - the operator that was named, the keys that
		/// were not read.
		Detail:String,
	},
	/// A lane serves no template, so it has no question to ask.
	LaneWithoutTemplate {
		/// The lane that serves nothing.
		Lane:String,
	},
	/// A template asks no question.
	TemplateAsksNothing {
		/// The template with no question.
		Template:String,
	},
	/// A question names no type, so nothing can say how to read its answer.
	QuestionWithoutType {
		/// The template the question belongs to.
		Template:String,
		/// The question's name inside that template.
		Question:String,
	},
	/// A choice offers no option, so no answer could be inside its criteria.
	ChoiceWithoutOption {
		/// The template the question belongs to.
		Template:String,
		/// The question's name inside that template.
		Question:String,
	},
	/// A score's scale has fewer than two words, so it has no position to sit at.
	ScoreWithoutScale {
		/// The template the question belongs to.
		Template:String,
		/// The question's name inside that template.
		Question:String,
	},
}

impl CorpusError {
	/// The file this failure is about, when it is about one file.
	///
	/// `None` for the refusals that are about the corpus as a whole - no lane
	/// anywhere, or a lane naming a template - because naming one of several
	/// files there would be a guess.
	pub fn Path(&self) -> Option<&Path> {
		match self {
			CorpusError::MissingFile { Path }
			| CorpusError::Io { Path, .. }
			| CorpusError::MalformedJSON { Path, .. }
			| CorpusError::UnexpectedShape { Path, .. } => Some(Path),
			_ => None,
		}
	}

	/// The port's own code for this failure.
	///
	/// The same split [`CorpusError::Unreadable`] makes, stated in the vocabulary
	/// `module.json` fixes: a corpus that could not be read at all, or one that
	/// was read and disagrees with itself. A caller that branches on the code gets
	/// the seven-code answer; a caller that wants to say which file was wrong
	/// keeps the `CorpusError` it came from.
	pub fn Code(&self) -> Code { if self.Unreadable() { Code::CorpusUnreadable } else { Code::CorpusInvalid } }

	/// Whether the corpus could not be read at all, as opposed to having been
	/// read and found to disagree with itself.
	///
	/// The same split the port's codes make (`corpus-unreadable` against
	/// `corpus-invalid`), stated here so a caller that only wants the summary
	/// does not have to match every variant.
	pub fn Unreadable(&self) -> bool {
		matches!(
			self,
			CorpusError::MissingFile { .. }
				| CorpusError::Io { .. }
				| CorpusError::MalformedJSON { .. }
				| CorpusError::UnexpectedShape { .. }
		)
	}

	/// The sentence a human reads, naming the path or the id that was wrong.
	pub fn Message(&self) -> String {
		match self {
			CorpusError::MissingFile { Path } => format!("{} is missing", Path.display()),
			CorpusError::Io { Path, .. } => format!("{} could not be read", Path.display()),
			CorpusError::MalformedJSON { Path, Role, .. } => format!("{Role} {} is not JSON", Path.display()),
			CorpusError::UnexpectedShape { Path, What, .. } => {
				format!("{}: {What} is not the shape the corpus layout puts there", Path.display())
			},
			CorpusError::NoLanes { .. } => "the corpus declares no lane".to_string(),
			CorpusError::DuplicateLane { Lane, .. } => format!("the lane {Lane} is declared twice"),
			CorpusError::UnknownTemplateRef { Lane, TemplateID, .. } => {
				format!("lane {Lane} names the template {TemplateID}, which does not exist")
			},
			CorpusError::UnknownFieldRef { Lane, Field, .. } => {
				format!("lane {Lane} sends the field {Field}, which the observation does not carry")
			},
			CorpusError::BadCondition { Lane, .. } => {
				format!("lane {Lane} states a condition this crate cannot read")
			},
			CorpusError::LaneWithoutTemplate { Lane } => {
				format!("lane {Lane} serves no template, so it has no question to ask")
			},
			CorpusError::TemplateAsksNothing { Template } => format!("template {Template} asks no question"),
			CorpusError::QuestionWithoutType { Template, Question } => {
				format!("the question {Template}/{Question} names no type")
			},
			CorpusError::ChoiceWithoutOption { Template, Question } => {
				format!("the choice {Template}/{Question} offers no option")
			},
			CorpusError::ScoreWithoutScale { Template, Question } => {
				format!("the score {Template}/{Question} names no scale: a position needs two words at least")
			},
		}
	}

	/// What was observed, kept apart from the sentence: the parser's complaint,
	/// the files that were searched, the id that was not found.
	pub fn Detail(&self) -> String {
		match self {
			CorpusError::MissingFile { Path } => {
				format!("the corpus layout requires the file {}", Path.display())
			},
			CorpusError::Io { Source, .. } => Source.to_string(),
			CorpusError::MalformedJSON { Source, .. } => Source.to_string(),
			CorpusError::UnexpectedShape { Found, .. } => Found.clone(),
			CorpusError::NoLanes { Searched } => Names(Searched),
			CorpusError::DuplicateLane { First, Second, .. } => format!("{} and {}", First.display(), Second.display()),
			CorpusError::UnknownTemplateRef { TemplateID, Available, .. } => {
				format!(
					"{TemplateID} is not one of the templates the corpus declares: {}",
					Listed(Available)
				)
			},
			CorpusError::UnknownFieldRef { Field, Available, .. } => {
				format!(
					"{Field} is not one of the fields the observation carries: {}",
					Listed(Available)
				)
			},
			CorpusError::BadCondition { Message, Detail, .. } => format!("{Message}: {Detail}"),
			CorpusError::LaneWithoutTemplate { Lane } => Lane.clone(),
			CorpusError::TemplateAsksNothing { Template } => Template.clone(),
			CorpusError::QuestionWithoutType { Template, Question }
			| CorpusError::ChoiceWithoutOption { Template, Question }
			| CorpusError::ScoreWithoutScale { Template, Question } => format!("{Template}/{Question}"),
		}
	}
}

impl fmt::Display for CorpusError {
	/// The message, so `{}` on this type is the sentence and never a debug dump.
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result { write!(Formatter, "{}", self.Message()) }
}

impl std::error::Error for CorpusError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			CorpusError::Io { Source, .. } => Some(Source),
			CorpusError::MalformedJSON { Source, .. } => Some(Source),
			_ => None,
		}
	}
}

/// The load-time failure as the port's own error, so a caller holding a
/// `Result<_, Error>` reaches the loader with `?`.
///
/// The split is the one [`CorpusError::Unreadable`] already makes: a corpus that
/// could not be read is `corpus-unreadable`, and a corpus that was read and
/// disagrees with itself is `corpus-invalid`. Nothing the port's shape can carry
/// is lost - what is lost is the `io::Error` itself, which is why the loader
/// returns [`CorpusError`] and this conversion is the *narrowing* rather than the
/// default.
impl From<CorpusError> for Error {
	fn from(Why:CorpusError) -> Self {
		let (Message, Detail) = (Why.Message(), Why.Detail());
		if Why.Unreadable() {
			Error::CorpusUnreadable(Message, Detail)
		} else {
			Error::CorpusInvalid(Message, Detail)
		}
	}
}

/// A list of names as one line, for an error that has to say what was available.
fn Listed(Items:&[String]) -> String {
	if Items.is_empty() {
		return "none".to_string();
	}
	Items.join(", ")
}

/// A list of paths as one line, which is what an error can carry.
fn Names(Paths:&[PathBuf]) -> String {
	if Paths.is_empty() {
		return "nothing".to_string();
	}
	Paths
		.iter()
		.map(|Path| Path.display().to_string())
		.collect::<Vec<String>>()
		.join(", ")
}
