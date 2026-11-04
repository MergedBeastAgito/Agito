//! Reading a corpus, and refusing one that disagrees with itself.
//!
//! The validation is the point. A corpus is two files written by hand, and the
//! failures that matter are the ones where the files are individually valid JSON
//! and jointly nonsense: a lane that sends a field the observation never
//! declared, a lane that serves a template nobody wrote, a choice with no
//! options, a score with a one-word scale. Every one of those composes into a
//! request the model cannot answer, and finding out here costs a `Result`
//! instead of finding out from an answer that quietly ignored half the state.
//!
//! Two entry points, because a corpus arrives two ways: [`Load`] takes the
//! directory a caller points at, [`Read`] takes the two texts. The second is
//! what a test uses, and what a caller with the files in memory uses - the
//! loader itself does not care where the bytes came from.

use std::{collections::BTreeMap, fs, path::Path};

use serde_json::{Map, Value};

use crate::Struct::{Corpus::Corpus, Error::Error, Lane::Lane, Question::Question, Template::Template};

/// The observation file's name inside a corpus directory.
pub const ObservationFile:&str = "Observation.json";

/// The question file's name inside a corpus directory.
pub const QuestionsFile:&str = "Questions.json";

/// Read the corpus in one directory.
pub fn Load(Dir:&Path) -> Result<Corpus, Error> {
	let Observation = Text(&Dir.join(ObservationFile), ObservationFile)?;
	let Questions = Text(&Dir.join(QuestionsFile), QuestionsFile)?;
	Read(&Observation, &Questions)
}

/// Read a corpus from the two texts, and validate it against itself.
pub fn Read(Observation:&str, Questions:&str) -> Result<Corpus, Error> {
	let Observed:Value = serde_json::from_str(Observation)
		.map_err(|Why| Error::CorpusUnreadable("the observation is not JSON", Why.to_string()))?;
	let Asked:Value = serde_json::from_str(Questions)
		.map_err(|Why| Error::CorpusUnreadable("the question file is not JSON", Why.to_string()))?;

	let Fields:BTreeMap<String, Value> = Object(Observed.get("state"), "the observation's state")?
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
	let Declared = Object(Asked.get("templates"), "the question file's templates")?;
	let mut Templates = BTreeMap::new();
	for (Name, Body) in Declared {
		Templates.insert(Name.clone(), ReadTemplate(Name, Body)?);
	}

	let Declared = Object(Asked.get("lanes"), "the question file's lanes")?;
	if Declared.is_empty() {
		return Err(Error::CorpusInvalid(
			"the question file declares no lane",
			"a corpus with no lane can select nothing",
		));
	}
	let mut Lanes = BTreeMap::new();
	for (Name, Body) in Declared {
		Lanes.insert(Name.clone(), ReadLane(Name, Body, &Fields, &Templates)?);
	}

	Ok(Corpus {
		Version:Asked.get("version").and_then(Value::as_u64).unwrap_or(0),
		Fields,
		Derived,
		Lanes,
		Templates,
	})
}

/// One file, as text, with the path in the detail rather than the sentence.
fn Text(Path:&Path, Name:&str) -> Result<String, Error> {
	fs::read_to_string(Path).map_err(|Why| {
		Error::CorpusUnreadable(format!("{Name} could not be read"), format!("{}: {Why}", Path.display()))
	})
}

/// One lane, checked against the observation and the templates.
fn ReadLane(
	Name:&str,
	Body:&Value,
	Fields:&BTreeMap<String, Value>,
	Templates:&BTreeMap<String, Template>,
) -> Result<Lane, Error> {
	let Sent = Strings(Body.get("fields"), &format!("lane {Name}'s fields"))?;
	for Field in &Sent {
		if !Fields.contains_key(Field) {
			return Err(Error::CorpusInvalid(
				format!("lane {Name} sends a field the observation does not carry"),
				format!("{Field} is not one of the observed fields: {}", Names(Fields)),
			));
		}
	}
	let Served = Strings(Body.get("templates"), &format!("lane {Name}'s templates"))?;
	if Served.is_empty() {
		return Err(Error::CorpusInvalid(
			format!("lane {Name} serves no template"),
			"a lane with no template has no question to ask",
		));
	}
	for One in &Served {
		if !Templates.contains_key(One) {
			return Err(Error::CorpusInvalid(
				format!("lane {Name} names a template that does not exist"),
				format!("{One} is not one of the templates: {}", Names(Templates)),
			));
		}
	}
	Ok(Lane {
		Name:Name.to_string(),
		Fields:Sent,
		Templates:Served,
		Cadence:Line(Body, "cadence"),
		Why:Line(Body, "why"),
	})
}

/// One template, with every question checked.
fn ReadTemplate(Name:&str, Body:&Value) -> Result<Template, Error> {
	let Declared = Object(Body.get("questions"), &format!("template {Name}'s questions"))?;
	if Declared.is_empty() {
		return Err(Error::CorpusInvalid(
			format!("template {Name} asks nothing"),
			"a template with no question composes into a request with no answer",
		));
	}
	let mut Questions = BTreeMap::new();
	for (Question, One) in Declared {
		Questions.insert(Question.clone(), ReadQuestion(Name, Question, One)?);
	}
	Ok(Template { Name:Name.to_string(), When:Line(Body, "when"), Questions })
}

/// One question: a choice, a score, or a type this crate carries unread.
fn ReadQuestion(Template:&str, Name:&str, Body:&Value) -> Result<Question, Error> {
	let Type = Body
		.get("type")
		.and_then(Value::as_str)
		.ok_or_else(|| Error::CorpusInvalid(format!("{Template}/{Name} names no type"), Body.to_string()))?;
	let Instructions = Line(Body, "instructions");
	match Type {
		"choice" => {
			let Declared = Object(Body.get("criteria"), &format!("{Template}/{Name}'s criteria"))?;
			let mut Criteria = BTreeMap::new();
			for (Option, Sentence) in Declared {
				Criteria.insert(Option.clone(), Sentence.as_str().unwrap_or_default().to_string());
			}
			if Criteria.is_empty() {
				return Err(Error::CorpusInvalid(
					format!("{Template}/{Name} offers no option"),
					"a choice with no criteria cannot be answered",
				));
			}
			Ok(Question::Choice { Instructions, Criteria })
		},
		"score" => {
			let Criteria = Strings(Body.get("criteria"), &format!("{Template}/{Name}'s criteria"))?;
			if Criteria.len() < 2 {
				return Err(Error::CorpusInvalid(
					format!("{Template}/{Name} is a score with no scale"),
					format!("{} criteria is not a scale: it needs at least two words", Criteria.len()),
				));
			}
			Ok(Question::Score { Instructions, Criteria })
		},
		Other => Ok(Question::Other { Type:Other.to_string(), Body:Body.clone() }),
	}
}

/// A field of a parsed object that must be an object.
fn Object<'a>(Value:Option<&'a Value>, What:&str) -> Result<&'a Map<String, Value>, Error> {
	Value.and_then(Value::as_object).ok_or_else(|| {
		Error::CorpusInvalid(
			format!("{What} is missing or is not an object"),
			Value.map(Value::to_string).unwrap_or_default(),
		)
	})
}

/// A field of a parsed object that must be an array of names.
///
/// Absent is empty rather than an error: a lane that sends no field is a lane
/// whose question is about the instructions alone, which is legal and useless in
/// equal measure, and the model can still answer it.
fn Strings(Value:Option<&Value>, What:&str) -> Result<Vec<String>, Error> {
	let Some(Value) = Value else {
		return Ok(Vec::new());
	};
	let Some(Items) = Value.as_array() else {
		return Err(Error::CorpusInvalid(format!("{What} is not an array"), Value.to_string()));
	};
	Items
		.iter()
		.map(|Item| {
			Item.as_str().map(str::to_string).ok_or_else(|| {
				Error::CorpusInvalid(format!("{What} holds something that is not a name"), Item.to_string())
			})
		})
		.collect()
}

/// A field of a parsed object that must be a string, or the empty string.
fn Line(Value:&Value, Key:&str) -> String { Value.get(Key).and_then(Value::as_str).unwrap_or_default().to_string() }

/// The keys of a table, for a refusal that has to say what was available.
fn Names<T>(Map:&BTreeMap<String, T>) -> String { Map.keys().cloned().collect::<Vec<String>>().join(", ") }
