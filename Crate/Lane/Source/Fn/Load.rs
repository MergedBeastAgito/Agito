//! Reading a corpus, and refusing one that disagrees with itself.
//!
//! # The layout, and why the loader is told a *root* rather than a file
//!
//! A corpus is a directory, not two paths a caller assembles:
//!
//! ```text
//! <root>/
//!   Templates/           optional: a corpus may be its templates directory
//!     Observation.json   every field a caller may send, with its descriptor
//!     Questions.json     the templates, and the lanes when they live here
//!   *.json               any further lane file the corpus keeps at its root
//! ```
//!
//! Which of the two shapes a caller has is answered by where `Observation.json`
//! is, never by a name: a root that holds a `Templates/Observation.json` is read
//! through it, and a root that holds the file itself is read as the templates
//! directory. A corpus is a directory of files somebody copied somewhere, and
//! the one that arrives without its wrapper is not a different corpus.
//!
//! The two template files are named, because the layout names them. The *lane*
//! files are not: which file a corpus keeps its lanes in is the corpus's
//! business, so the loader reads the directory and takes every JSON file that
//! carries a `lanes` object - whether that is the question file itself (the
//! single-file shape) or one or more files beside it. A corpus that names its
//! lanes file `Lanes.json` and one that names it `shape.json` load the same way,
//! because no file name is consulted.
//!
//! Discovery is one level deep, in the root and in `Templates/`, and it is
//! strict: a JSON file there that does not parse is a refusal rather than a file
//! silently skipped. Skipping it is the worse failure - the corpus loads with
//! half its lanes because one file had a trailing comma, and the damage surfaces
//! at run time as a lane that "does not exist".
//!
//! # The validation is the point
//!
//! A corpus is written by hand, and the failures that matter are the ones where
//! the files are individually valid JSON and jointly nonsense: a lane that sends
//! a field the observation neither reads nor derives, a lane that serves a
//! template nobody wrote, a choice with no options, a score with a one-word
//! scale. Every one of those composes into a request the model cannot answer,
//! and finding out here costs a [`CorpusError`] instead of finding out from an
//! answer that quietly ignored half the state.
//!
//! # What a lane carries
//!
//! A lane names the fields it sends, the templates it serves, and - when the
//! corpus declares one - the condition that selects it. The condition is read
//! here and interpreted by [`crate::Fn::Condition::Condition`], so a lane is
//! resolved at load time in every part that can be: an unknown template id, an
//! unreadable condition and a field the observation does not name are all
//! refusals now, rather than a request composed from half a lane later.
//!
//! A lane may send a *derived* field as well as a read one, and it must be
//! allowed to: the honest lane sends the sentence that says what the numbers
//! mean beside the numbers, and that sentence is computed by the ingestion
//! rather than read from it. Refusing it would refuse the lanes that work best.
//!
//! # Two entry points, one assembly
//!
//! [`Load`] takes the directory a caller points at; [`Read`] takes the two texts
//! a caller already has in memory. Both go through the same assembly, so the
//! validation cannot differ between a corpus read from disk and a corpus read
//! from a string - the loader itself does not care where the bytes came from,
//! only what they say.

use std::{
	collections::BTreeMap,
	fs,
	io,
	path::{Path, PathBuf},
};

use serde_json::{Map, Value};

use crate::Struct::{
	Corpus::Corpus,
	CorpusError::CorpusError,
	Lane::Lane,
	Predicate::Predicate,
	Question::Question,
	Template::Template,
};

/// The directory inside a corpus root that holds the two template files.
pub const TemplatesDir:&str = "Templates";

/// The observation file's name inside the templates directory.
pub const ObservationFile:&str = "Observation.json";

/// The questions file's name inside the templates directory.
pub const QuestionsFile:&str = "Questions.json";

/// One text, and the name a refusal calls it by.
///
/// A text read from disk carries its path; a text a caller handed over carries
/// the file name it would have had. Both are reported the same way, so a refusal
/// from [`Read`] is as readable as one from [`Load`].
struct Named {
	/// Where the text came from.
	Path:PathBuf,
	/// The text itself.
	Text:String,
}

/// Read the corpus rooted at one directory.
///
/// The root is the corpus's own directory, or the templates directory itself: a
/// root that holds a `Templates/Observation.json` is read through it, and one
/// that holds the file is read where it is. Any further JSON file at the root or
/// in `Templates/` that declares lanes is read as one. Nothing about the corpus
/// is baked in except the layout itself.
pub fn Load(Root:&Path) -> Result<Corpus, CorpusError> {
	let Templates = if Root.join(TemplatesDir).join(ObservationFile).is_file() {
		Root.join(TemplatesDir)
	} else {
		Root.to_path_buf()
	};
	let Observation = Named {
		Path:Templates.join(ObservationFile),
		Text:Text(&Templates.join(ObservationFile))?,
	};
	let Questions = Named { Path:Templates.join(QuestionsFile), Text:Text(&Templates.join(QuestionsFile))? };

	// Every other JSON file the layout puts beside the templates. The two files
	// above are skipped by path, not by name, so a corpus that keeps its lanes
	// in a second file is read the same way as one that keeps them inline.
	let mut Lanes = Vec::new();
	for Directory in Directories(Root, &Templates) {
		for Path in Candidates(&Directory)? {
			if Path == Observation.Path || Path == Questions.Path {
				continue;
			}
			Lanes.push(Named { Path:Path.clone(), Text:Text(&Path)? });
		}
	}

	Assemble(Observation, Questions, Lanes)
}

/// The directories a corpus's lane files may sit in, root first.
///
/// One entry when the root *is* the templates directory, so a flat corpus is not
/// scanned twice and a lane is not declared twice by the same file.
fn Directories(Root:&Path, Templates:&Path) -> Vec<PathBuf> {
	if Root == Templates {
		vec![Templates.to_path_buf()]
	} else {
		vec![Root.to_path_buf(), Templates.to_path_buf()]
	}
}

/// Read a corpus from the two texts, and validate it against itself.
///
/// The lane sources are whatever the question text carries: a caller holding the
/// two files in memory holds the lanes with them. A caller that also has a
/// separate lane file reads it from disk with [`Load`].
pub fn Read(Observation:&str, Questions:&str) -> Result<Corpus, CorpusError> {
	Assemble(
		Named { Path:PathBuf::from(ObservationFile), Text:Observation.to_string() },
		Named { Path:PathBuf::from(QuestionsFile), Text:Questions.to_string() },
		Vec::new(),
	)
}

/// The one assembly: two named texts, plus whatever further lane files exist.
fn Assemble(Observation:Named, Questions:Named, Extra:Vec<Named>) -> Result<Corpus, CorpusError> {
	let Observed = Json(&Observation)?;
	let Asked = Json(&Questions)?;

	let Fields:BTreeMap<String, Value> = Object(Observed.get("state"), &Observation.Path, "the observation's state")?
		.iter()
		.map(|(Name, Descriptor)| (Name.clone(), Descriptor.clone()))
		.collect();
	let Derived:BTreeMap<String, Value> = Observed
		.get("derived")
		.and_then(Value::as_object)
		.map(|Derived| Derived.iter().map(|(Name, One)| (Name.clone(), One.clone())).collect())
		.unwrap_or_default();

	// Templates first: a lane is validated against the templates it serves, so
	// the table has to exist before a lane can be checked.
	let Declared = Object(Asked.get("templates"), &Questions.Path, "the question file's templates")?;
	let mut Templates = BTreeMap::new();
	for (Name, Body) in Declared {
		Templates.insert(Name.clone(), ReadTemplate(Name, Body, &Questions.Path)?);
	}

	// The lane sources: the question file when it carries lanes, plus every
	// other JSON file the layout put beside the templates. A file that declares
	// no lane contributes nothing rather than being an error, because a corpus
	// directory may hold other JSON.
	let mut Sources:Vec<(&PathBuf, Value)> = Vec::new();
	let mut Searched:Vec<PathBuf> = vec![Questions.Path.clone()];
	if Asked.get("lanes").is_some() {
		Sources.push((&Questions.Path, Asked.clone()));
	}
	for One in &Extra {
		let Parsed = Json(One)?;
		Searched.push(One.Path.clone());
		if Parsed.get("lanes").is_some() {
			Sources.push((&One.Path, Parsed));
		}
	}

	let mut Lanes = BTreeMap::new();
	let mut Origin:BTreeMap<String, PathBuf> = BTreeMap::new();
	for (Source, Body) in Sources {
		let Declared = Object(Body.get("lanes"), Source, "the lane table")?;
		for (Name, One) in Declared {
			if let Some(First) = Origin.get(Name) {
				return Err(CorpusError::DuplicateLane {
					lane:Name.clone(),
					first:First.clone(),
					second:Source.clone(),
				});
			}
			Origin.insert(Name.clone(), Source.clone());
			Lanes.insert(Name.clone(), ReadLane(Name, One, Source, &Fields, &Derived, &Templates)?);
		}
	}
	if Lanes.is_empty() {
		return Err(CorpusError::NoLanes { searched:Searched });
	}

	Ok(Corpus {
		Version:Asked.get("version").and_then(Value::as_u64).unwrap_or(0),
		Fields,
		Derived,
		Lanes,
		Templates,
	})
}

/// One file, as text, with a missing file distinguished from an unreadable one.
///
/// The two are different answers: a missing file is a corpus that was never
/// copied there, while an unreadable one is a corpus that is there and whose
/// permissions or type are wrong.
fn Text(Path:&Path) -> Result<String, CorpusError> { fs::read_to_string(Path).map_err(|Why| Problem(Path, Why)) }

/// Every `*.json` file directly inside one directory, in a stable order.
///
/// Sorted, because `read_dir` order is the file system's, and a refusal that
/// names a file has to name the same file twice.
fn Candidates(Directory:&Path) -> Result<Vec<PathBuf>, CorpusError> {
	let mut Paths:Vec<PathBuf> = Vec::new();
	for Entry in fs::read_dir(Directory).map_err(|Why| Problem(Directory, Why))? {
		let Entry = Entry.map_err(|Why| Problem(Directory, Why))?;
		let Path = Entry.path();
		if Path.is_file() && Path.extension().is_some_and(|Extension| Extension.eq_ignore_ascii_case("json")) {
			Paths.push(Path);
		}
	}
	Paths.sort();
	Ok(Paths)
}

/// One named text as JSON.
fn Json(Source:&Named) -> Result<Value, CorpusError> {
	serde_json::from_str(&Source.Text)
		.map_err(|Why| CorpusError::MalformedJson { path:Source.Path.clone(), source:Why })
}

/// An operating-system refusal, with a missing path kept apart from the rest.
fn Problem(Path:&Path, Why:io::Error) -> CorpusError {
	if Why.kind() == io::ErrorKind::NotFound {
		CorpusError::MissingFile { path:Path.to_path_buf() }
	} else {
		CorpusError::Io { path:Path.to_path_buf(), source:Why }
	}
}

/// One lane, checked against the observation and the templates.
fn ReadLane(
	Name:&str,
	Body:&Value,
	Source:&Path,
	Fields:&BTreeMap<String, Value>,
	Derived:&BTreeMap<String, Value>,
	Templates:&BTreeMap<String, Template>,
) -> Result<Lane, CorpusError> {
	let Sent = Strings(Body.get("fields"), Source, &format!("lane {Name}'s fields"))?;
	for Field in &Sent {
		// A derived condition is as sendable as a read field: the lane's job is
		// to send what the question reads, and `verdict` is what it reads.
		if !Fields.contains_key(Field) && !Derived.contains_key(Field) {
			return Err(CorpusError::UnknownFieldRef { lane:Name.to_string(), field:Field.clone() });
		}
	}
	let Served = Strings(Body.get("templates"), Source, &format!("lane {Name}'s templates"))?;
	if Served.is_empty() {
		return Err(CorpusError::LaneWithoutTemplate { lane:Name.to_string() });
	}
	for One in &Served {
		if !Templates.contains_key(One) {
			return Err(CorpusError::UnknownTemplateRef { lane:Name.to_string(), template_id:One.clone() });
		}
	}
	// The lane's own condition. Reading it here is what makes a lane pure data:
	// the corpus says when it applies, so an adapter adds one without this crate
	// being rebuilt, and a lane that says nothing applies when nothing more
	// specific does.
	let Select = match Body.get("select").filter(|One| !One.is_null()) {
		Some(Declared) => ReadSelect(Name, Declared, Source, Fields, Derived)?,
		None => Predicate::Always,
	};
	Ok(Lane {
		Name:Name.to_string(),
		// Kept as the corpus spelled it (empty when it named none): normalising
		// here would erase the difference between "this lane asks clef" and
		// "this lane asks whatever the caller's default is", which is the one
		// thing a caller overriding the model needs to know.
		Model:Line(Body, "model"),
		Fields:Sent,
		Templates:Served,
		Select,
		Cadence:Line(Body, "cadence"),
		Why:Line(Body, "why"),
	})
}

/// One lane's condition, read from the corpus and checked against the fields
/// the corpus names.
///
/// A condition over a field neither the observation nor its derived table names
/// is a lane that can never hold - a typo in a corpus, not a caller's mistake -
/// and finding that out at load is cheaper than wondering why the lane never
/// appears. `and` with no cases is the same statement as `always`, and one
/// statement spelled two ways is a corpus two authors can disagree in, so it is
/// read as the statement it means.
fn ReadSelect(
	Lane:&str,
	Declared:&Value,
	Source:&Path,
	Fields:&BTreeMap<String, Value>,
	Derived:&BTreeMap<String, Value>,
) -> Result<Predicate, CorpusError> {
	let Condition:Predicate = serde_json::from_value(Declared.clone()).map_err(|Why| {
		CorpusError::UnexpectedShape {
			path:Source.to_path_buf(),
			what:format!("lane {Lane}'s select"),
			found:Why.to_string(),
		}
	})?;
	for Field in Condition.Fields() {
		if !Fields.contains_key(Field) && !Derived.contains_key(Field) {
			return Err(CorpusError::UnknownFieldRef { lane:Lane.to_string(), field:Field.to_string() });
		}
	}
	Ok(match Condition {
		Predicate::And { Of } if Of.is_empty() => Predicate::Always,
		Other => Other,
	})
}

/// One template, with every question checked.
fn ReadTemplate(Name:&str, Body:&Value, Source:&Path) -> Result<Template, CorpusError> {
	let Declared = Object(Body.get("questions"), Source, &format!("template {Name}'s questions"))?;
	if Declared.is_empty() {
		return Err(CorpusError::TemplateAsksNothing { template:Name.to_string() });
	}
	let mut Questions = BTreeMap::new();
	for (Question, One) in Declared {
		Questions.insert(Question.clone(), ReadQuestion(Name, Question, One, Source)?);
	}
	Ok(Template { Name:Name.to_string(), When:Line(Body, "when"), Questions })
}

/// One question: a choice, a score, or a type this crate carries unread.
fn ReadQuestion(Template:&str, Name:&str, Body:&Value, Source:&Path) -> Result<Question, CorpusError> {
	let Type = Body
		.get("type")
		.and_then(Value::as_str)
		.ok_or_else(|| CorpusError::QuestionWithoutType { template:Template.to_string(), question:Name.to_string() })?;
	let Instructions = Line(Body, "instructions");
	match Type {
		"choice" => {
			let Declared = Object(Body.get("criteria"), Source, &format!("{Template}/{Name}'s criteria"))?;
			let mut Criteria = BTreeMap::new();
			for (Option, Sentence) in Declared {
				Criteria.insert(Option.clone(), Sentence.as_str().unwrap_or_default().to_string());
			}
			if Criteria.is_empty() {
				return Err(CorpusError::ChoiceWithoutOption {
					template:Template.to_string(),
					question:Name.to_string(),
				});
			}
			Ok(Question::Choice { Instructions, Criteria })
		},
		"score" => {
			let Criteria = Strings(Body.get("criteria"), Source, &format!("{Template}/{Name}'s criteria"))?;
			if Criteria.len() < 2 {
				return Err(CorpusError::ScoreWithoutScale {
					template:Template.to_string(),
					question:Name.to_string(),
				});
			}
			Ok(Question::Score { Instructions, Criteria })
		},
		Other => Ok(Question::Other { Type:Other.to_string(), Body:Body.clone() }),
	}
}

/// A field of a parsed object that must be an object.
fn Object<'a>(Value:Option<&'a Value>, Path:&Path, What:&str) -> Result<&'a Map<String, Value>, CorpusError> {
	Value.and_then(Value::as_object).ok_or_else(|| {
		CorpusError::UnexpectedShape {
			path:Path.to_path_buf(),
			what:What.to_string(),
			found:Value.map(Kind).unwrap_or("missing").to_string(),
		}
	})
}

/// A field of a parsed object that must be an array of names.
///
/// Absent is empty rather than an error: a lane that sends no field is a lane
/// whose question is about the instructions alone, which is legal and useless in
/// equal measure, and the model can still answer it.
fn Strings(Value:Option<&Value>, Path:&Path, What:&str) -> Result<Vec<String>, CorpusError> {
	let Some(Value) = Value else {
		return Ok(Vec::new());
	};
	let Some(Items) = Value.as_array() else {
		return Err(CorpusError::UnexpectedShape {
			path:Path.to_path_buf(),
			what:What.to_string(),
			found:Kind(Value).to_string(),
		});
	};
	Items
		.iter()
		.map(|Item| {
			Item.as_str().map(str::to_string).ok_or_else(|| {
				CorpusError::UnexpectedShape {
					path:Path.to_path_buf(),
					what:format!("an entry of {What}"),
					found:Item.to_string(),
				}
			})
		})
		.collect()
}

/// What a JSON value is, for a refusal that has to say what it found.
fn Kind(Value:&Value) -> &'static str {
	match Value {
		Value::Null => "null",
		Value::Bool(_) => "a boolean",
		Value::Number(_) => "a number",
		Value::String(_) => "a string",
		Value::Array(_) => "an array",
		Value::Object(_) => "an object",
	}
}

/// A field of a parsed object that must be a string, or the empty string.
fn Line(Value:&Value, Key:&str) -> String { Value.get(Key).and_then(Value::as_str).unwrap_or_default().to_string() }
