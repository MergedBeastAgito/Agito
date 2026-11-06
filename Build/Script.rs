// Compiling a crate's `Source/Script/**/*.ts` into the JavaScript it sends.
//
// Included by a crate's `build.rs` - `Crate/Chat/Driver/build.rs` and
// `Crate/Workflow/build.rs` are each three lines around it - so the rule for
// what a script may contain and how it is turned into bytes is written once.
// The comments are `//` rather than `//!` because an `include!`d file cannot
// carry an inner doc comment: it lands after the including file's first item.
//
// # What compiling means here
//
// The scripts are not bundled, minified or re-printed. Every transpiler that
// generates code from an AST rewrites the whole file: OXC's own transform
// appends semicolons, re-indents and re-attaches comments, SWC and `deno_ast`
// do the same, `esbuild` drops comment blocks and rewrites `undefined`, and
// `tsc` prepends `"use strict";`. Any of those turns a page expression that
// was verified byte for byte into a different one, and turns "this diff is a
// change in what the page receives" into "this diff is a change in a
// formatter's opinion".
//
// So the compilation is the smallest transformation that makes TypeScript
// into JavaScript: the file is parsed by a real TypeScript parser, the spans
// of the TypeScript-only syntax are found in the AST, and exactly those bytes
// are removed. Every other byte - indentation, quotes, comments, semicolons -
// is the byte the page already received. A file with no type syntax at all
// compiles to itself, which is why the move to `.ts` changed no expression.
//
// # What is checked, and what is not
//
// Checked at build time, and a build error when it fails:
//
// - the file parses as TypeScript (a syntax error names the file and line);
// - every TypeScript construct in it is one this file knows how to erase - an
//   unrecognised one is a build error rather than a silently mangled
//   expression;
// - the erased result parses as JavaScript, so an erasure that ate a bracket
//   cannot reach a page;
// - every `__PARAM_Name__` marker has a `declare const` of the same name, and
//   every declaration is used - the parameter contract cannot drift from the
//   expression it describes.
//
// Not checked: types. A transpiler parses, it does not infer, so
// `declare const __PARAM_Scroll__: string;` is a declaration a type checker
// would hold the caller to, and nothing here does. `tsc --noEmit` over these
// sources is a manual check, recorded in
// `Documentation/Reference/Script/Compilation.md`; the build never needs one.
//
// # Determinism
//
// The output is a pure function of the source: no timestamps, no absolute
// paths, no iteration over a `HashMap`. A file whose compiled bytes are
// unchanged is not rewritten, so a rebuild after a no-op edit leaves the
// artefact's mtime alone and nothing downstream rebuilds.

use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};

/// The marker a placeholder is written as in a script.
///
/// It is an identifier rather than `{{name}}` for one reason: an identifier is
/// valid TypeScript, so the file the parser sees is the file the page's
/// expression is made of. `{{name}}` is not parseable, which is what made the
/// old templates impossible to compile.
const Marker:&str = "__PARAM_";

/// Whether a parameter is data or code.
///
/// The kind is read off the declaration: `declare const __PARAM_X__: string;`
/// is data and is substituted as an escaped JavaScript string literal; every
/// other declared type is code and is spliced as it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
	Text,
	Code,
}

impl Kind {
	fn Name(self) -> &'static str {
		match self {
			Kind::Text => "text",
			Kind::Code => "code",
		}
	}
}

/// A parameter a script declares.
#[derive(Debug, Clone)]
pub struct Parameter {
	/// The name as a call site writes it: `Scroll` for `__PARAM_Scroll__`.
	pub Name:String,
	pub Kind:Kind,
}

/// One compiled script.
pub struct Compiled {
	pub JavaScript:String,
	pub Parameters:Vec<Parameter>,
}

/// Why a script could not be compiled, with the position to name.
pub struct Failure {
	pub File:PathBuf,
	pub Line:usize,
	pub Column:usize,
	pub Message:String,
}

impl std::fmt::Display for Failure {
	fn fmt(&self, Formatter:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			Formatter,
			"{}:{}:{}: {}",
			self.File.display(),
			self.Line,
			self.Column,
			self.Message
		)
	}
}

/// The 1-based line and column of a byte offset, and the text of its line.
fn Position(Text:&str, Offset:u32) -> (usize, usize) {
	let Offset = (Offset as usize).min(Text.len());
	let Before = &Text[..Offset];
	let Line = Before.bytes().filter(|Byte| *Byte == b'\n').count() + 1;
	let Start = Before.rfind('\n').map(|At| At + 1).unwrap_or(0);
	(Line, Offset - Start + 1)
}

/// A span the compiler removes, plus what it learned from it.
struct Erase<'t> {
	Cuts:Vec<Span>,
	Declared:Vec<Parameter>,
	Rejected:Vec<(u32, String)>,
	Text:&'t str,
}

impl<'t> Erase<'t> {
	/// Remove a whole line, and the blank line that follows it when there is
	/// one: a declaration block is separated from the expression by one blank
	/// line, and that line belongs to the block.
	fn Line(&mut self, Span:Span) {
		let Text = self.Text;
		let mut End = Span.end as usize;
		while End < Text.len() && Text.as_bytes()[End] != b'\n' {
			End += 1;
		}
		if End < Text.len() {
			End += 1;
		}
		if End < Text.len() && Text.as_bytes()[End] == b'\n' {
			End += 1;
		}
		self.Cuts.push(Span::new(Span.start, End as u32));
	}
}

impl<'a> Visit<'a> for Erase<'_> {
	fn visit_statement(&mut self, It:&Statement<'a>) {
		match It {
			// `declare const __PARAM_Name__: T;` - the parameter contract. It is
			// read for the name and the kind, and removed from the expression.
			Statement::VariableDeclaration(Declaration) if Declaration.declare => {
				for Declarator in &Declaration.declarations {
					let Ident = match &Declarator.id {
						BindingPattern::BindingIdentifier(Identifier) => Identifier.name.as_str(),
						_ => {
							self.Rejected.push((
								It.span().start,
								"a declaration this build does not read: bind by name".to_string(),
							));
							continue;
						},
					};
					let Name = match Ident.strip_prefix(Marker).and_then(|Rest| Rest.strip_suffix("__")) {
						Some(Name) if !Name.is_empty() => Name.to_string(),
						_ => {
							self.Rejected.push((
								It.span().start,
								format!(
									"`declare const {Ident}` is not a script parameter: the form is `__PARAM_Name__`"
								),
							));
							continue;
						},
					};
					let Kind = match Declarator.type_annotation.as_ref().map(|A| &A.type_annotation) {
						Some(TSType::TSStringKeyword(_)) => Kind::Text,
						Some(_) => Kind::Code,
						None => {
							self.Rejected.push((
								It.span().start,
								format!("`{Ident}` declares no type, so it is neither data nor code"),
							));
							continue;
						},
					};
					self.Declared.push(Parameter { Name, Kind });
				}
				self.Line(It.span());
				return;
			},
			// TypeScript-only statements. Erasing them by span is only safe for
			// the ones the scripts actually use, so the rest are refused rather
			// than guessed at.
			Statement::TSTypeAliasDeclaration(_)
			| Statement::TSInterfaceDeclaration(_)
			| Statement::TSEnumDeclaration(_)
			| Statement::TSNamespaceDeclaration(_)
			| Statement::TSGlobalDeclaration(_)
			| Statement::TSExternalModuleDeclaration(_)
			| Statement::TSImportEqualsDeclaration(_) => {
				self.Rejected.push((
					It.span().start,
					"a TypeScript-only statement this build cannot erase: put the type in a parameter declaration"
						.to_string(),
				));
				return;
			},
			_ => {},
		}
		walk::walk_statement(self, It);
	}

	/// `x: T` - the annotation, colon included, is not JavaScript.
	fn visit_ts_type_annotation(&mut self, It:&TSTypeAnnotation<'a>) { self.Cuts.push(It.span); }

	/// `x as T` - the assertion, keyword included, is not JavaScript.
	fn visit_ts_as_expression(&mut self, It:&TSAsExpression<'a>) {
		self.Cuts
			.push(Span::new(It.expression.span().end, It.type_annotation.span().end));
		walk::walk_ts_as_expression(self, It);
	}

	/// Everything else TypeScript-only, refused by name.
	fn visit_ts_satisfies_expression(&mut self, It:&TSSatisfiesExpression<'a>) {
		self.Rejected.push((
			It.span.start,
			"`satisfies` is not erased by this build: use `as` or a declaration".to_string(),
		));
	}

	fn visit_ts_non_null_expression(&mut self, It:&TSNonNullExpression<'a>) {
		self.Rejected.push((
			It.span.start,
			"a non-null assertion is not erased by this build: check for `null` instead".to_string(),
		));
	}

	fn visit_ts_type_assertion(&mut self, It:&TSTypeAssertion<'a>) {
		self.Rejected
			.push((It.span.start, "`<T>x` is not erased by this build: use `x as T`".to_string()));
	}

	fn visit_ts_type_parameter_declaration(&mut self, It:&TSTypeParameterDeclaration<'a>) {
		self.Rejected.push((
			It.span.start,
			"generic type parameters are not erased by this build".to_string(),
		));
	}

	fn visit_ts_instantiation_expression(&mut self, It:&TSInstantiationExpression<'a>) {
		self.Rejected
			.push((It.span.start, "a type instantiation is not erased by this build".to_string()));
	}
}

/// Apply the cuts, in source order, keeping every byte outside them.
fn Cut(Text:&str, Cuts:&[Span]) -> String {
	let mut Sorted = Cuts.to_vec();
	Sorted.sort_by_key(|Span| Span.start);
	let mut Out = String::with_capacity(Text.len());
	let mut At = 0usize;
	for Span in Sorted {
		let (Start, End) = (Span.start as usize, Span.end as usize);
		if Start < At {
			continue;
		}
		Out.push_str(&Text[At..Start]);
		At = End;
	}
	Out.push_str(&Text[At..]);
	Out
}

/// Compile one script: parse it as TypeScript, erase the type syntax, and check
/// that what is left is JavaScript and that its markers match its declarations.
pub fn Compile(File:&Path, Text:&str) -> Result<Compiled, Failure> {
	let Fail = |Offset:u32, Message:String| {
		let (Line, Column) = Position(Text, Offset);
		Failure { File:File.to_path_buf(), Line, Column, Message }
	};

	let SourceType = SourceType::from_path(File).unwrap_or_default();
	if !SourceType.is_typescript() {
		return Err(Fail(0, "not a TypeScript file".to_string()));
	}

	let Allocator = Allocator::default();
	let Parsed = Parser::new(&Allocator, Text, SourceType).parse();
	if let Some(Diagnostic) = Parsed.diagnostics.first() {
		let Offset = Diagnostic.labels.first().map(|Label| Label.span().start).unwrap_or(0);
		return Err(Fail(Offset, Diagnostic.message.to_string()));
	}

	let mut Pass = Erase { Cuts:Vec::new(), Declared:Vec::new(), Rejected:Vec::new(), Text };
	Pass.visit_program(&Parsed.program);
	if let Some((Offset, Message)) = Pass.Rejected.first() {
		return Err(Fail(*Offset, Message.clone()));
	}

	let JavaScript = Cut(Text, &Pass.Cuts);

	// The erasure has to leave JavaScript. A cut that took a bracket with it is
	// the failure this catches, and it catches it before a page does.
	let AsJavaScript = SourceType::from_path(Path::new("erased.js")).unwrap_or_default();
	let Check = Parser::new(&Allocator, &JavaScript, AsJavaScript).parse();
	if let Some(Diagnostic) = Check.diagnostics.first() {
		let Offset = Diagnostic.labels.first().map(|Label| Label.span().start).unwrap_or(0);
		return Err(Fail(
			Offset,
			format!("the erased script is not JavaScript: {}", Diagnostic.message),
		));
	}

	// Every marker the expression carries is declared, and every declaration is
	// used. A marker without a declaration has no kind, so the build cannot say
	// whether it should be escaped; a declaration without a marker is a
	// parameter that would be checked at a call site and go nowhere.
	let mut Seen = Vec::new();
	let mut Rest = JavaScript.as_str();
	while let Some(At) = Rest.find(Marker) {
		let After = &Rest[At + Marker.len()..];
		let End = match After.find("__") {
			Some(End) => End,
			None => {
				let Offset = (JavaScript.len() - Rest.len() + At) as u32;
				return Err(Fail(Offset, "an unterminated `__PARAM_` marker".to_string()));
			},
		};
		let Name = After[..End].to_string();
		if !Seen.contains(&Name) {
			Seen.push(Name);
		}
		Rest = &After[End..];
	}
	for Name in &Seen {
		if !Pass.Declared.iter().any(|Parameter| &Parameter.Name == Name) {
			let At = JavaScript.find(&format!("{Marker}{Name}__")).unwrap_or(0) as u32;
			return Err(Fail(
				At,
				format!("`{Marker}{Name}__` is used but not declared: add `declare const {Marker}{Name}__: <type>;`"),
			));
		}
	}
	for Parameter in &Pass.Declared {
		if !Seen.contains(&Parameter.Name) {
			let At = Text.find(&format!("{Marker}{}__", Parameter.Name)).unwrap_or(0) as u32;
			return Err(Fail(At, format!("`{Marker}{}__` is declared but never used", Parameter.Name)));
		}
	}
	let mut Declared = Vec::new();
	for Parameter in &Pass.Declared {
		if Declared.iter().any(|Seen:&Parameter| Seen.Name == Parameter.Name) {
			let At = Text.find(&format!("{Marker}{}__", Parameter.Name)).unwrap_or(0) as u32;
			return Err(Fail(At, format!("`{}` is declared twice", Parameter.Name)));
		}
		Declared.push(Parameter.clone());
	}

	Ok(Compiled { JavaScript, Parameters:Declared })
}

/// Write `Bytes` to `Path` only when they differ, so a rebuild that changes
/// nothing leaves the artefact - and everything built on it - alone.
fn Write(Path:&Path, Bytes:&str) -> std::io::Result<()> {
	if let Ok(Existing) = std::fs::read(Path)
		&& Existing == Bytes.as_bytes()
	{
		return Ok(());
	}
	std::fs::write(Path, Bytes)
}

/// Every `.ts` under `Root`, in a stable order.
fn Sources(Root:&Path) -> Vec<PathBuf> {
	let mut Found = Vec::new();
	let mut Pending = vec![Root.to_path_buf()];
	while let Some(Directory) = Pending.pop() {
		let Ok(Entries) = std::fs::read_dir(&Directory) else {
			continue;
		};
		for Entry in Entries.flatten() {
			let Path = Entry.path();
			if Path.is_dir() {
				Pending.push(Path);
			} else if Path.extension().is_some_and(|Extension| Extension == "ts") {
				Found.push(Path);
			}
		}
	}
	Found.sort();
	Found
}

/// Compile every script under `Source` into `Out`, and tell Cargo to rebuild
/// when one of them changes. Called from a crate's `build.rs`.
///
/// A failure is printed as `file:line:column: message` and exits non-zero, so a
/// malformed script stops the build with the position named rather than
/// reaching a page.
pub fn CompileScripts(Source:&str, Out:&str) {
	let Root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
	let Source = Root.join(Source);
	let Out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join(Out);

	println!("cargo:rerun-if-changed={}", Source.display());

	let Mut = Sources(&Source);
	if Mut.is_empty() {
		println!("cargo:warning=no scripts under {}: nothing to compile", Source.display());
	}

	for File in Mut {
		let Text = std::fs::read_to_string(&File)
			.unwrap_or_else(|Error| Stop(&format!("{}: cannot be read: {Error}", Shown(&File, &Root))));
		let Compiled = Compile(&File, &Text).unwrap_or_else(|Failure| {
			// The path as a reader of this crate knows it: `Source/Script/...`,
			// not the absolute path a build runs in.
			Stop(&format!(
				"{}:{}:{}: {}",
				Shown(&Failure.File, &Root),
				Failure.Line,
				Failure.Column,
				Failure.Message
			))
		});

		// The output keeps the source's relative path, minus the extension, so
		// `Source/Script/Expression/ReadAnswers.ts` is
		// `script!("Expressions/ReadAnswers")`.
		let Relative = File.strip_prefix(&Source).unwrap_or(&File).with_extension("");
		let JavaScript = Out.join(&Relative).with_extension("js");
		let Parameters = Out.join(&Relative).with_extension("params");
		if let Some(Parent) = JavaScript.parent() {
			std::fs::create_dir_all(Parent)
				.unwrap_or_else(|Error| Stop(&format!("{}: cannot be created: {Error}", Parent.display())));
		}

		Write(&JavaScript, &Compiled.JavaScript)
			.unwrap_or_else(|Error| Stop(&format!("{}: cannot be written: {Error}", JavaScript.display())));

		let Declared = Compiled
			.Parameters
			.iter()
			.map(|Parameter| format!("{}:{}\n", Parameter.Name, Parameter.Kind.Name()))
			.collect::<String>();
		Write(&Parameters, &Declared)
			.unwrap_or_else(|Error| Stop(&format!("{}: cannot be written: {Error}", Parameters.display())));
	}
}

/// `Path` as a reader of this crate knows it, when it is inside the crate.
fn Shown(Path:&Path, Root:&Path) -> String { Path.strip_prefix(Root).unwrap_or(Path).display().to_string() }

/// Stop the build with one line a reader can act on.
///
/// A build script that panics buries the position in a backtrace header; the
/// point of naming the file and the line is that the reader goes there, so the
/// message is the error and the exit status is the failure.
fn Stop<T>(Message:&str) -> T {
	eprintln!("error: {Message}");
	std::process::exit(1);
}
