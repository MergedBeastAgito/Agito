//! Every diagram is a typed source file with a rendered sibling.
//!
//! This is `Scripts/Legend.mjs` in Rust, and it is the one gate that moved
//! without changing its subject: the drawings under `Documentation/Diagram/`
//! are still Mermaid sources with an `.svg` beside them, and this still refuses
//! the four things that make a drawing unreadable to the next person - no
//! frontmatter, no line9 theme, no class annotation, and a prose legend inside
//! the drawing that the annotations were meant to replace.
//!
//! It is a test rather than a script because the gate it replaces was a command
//! somebody had to remember: `cargo test --workspace` runs it, so a page added
//! without a legend fails the same push that adds it.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	fs,
	path::{Path, PathBuf},
};

/// The classes the shared line9 theme defines. A drawing that annotates no node
/// with one of them is a drawing whose shapes mean nothing to a reader who has
/// seen the other pages.
const Classes:[&str; 4] = [":::host", ":::hook", ":::store", ":::fail"];

/// A rendered `.svg` under this many bytes is a placeholder or a failed render,
/// not a drawing.
const MinimumSvgBytes:u64 = 1000;

/// The prose marker the annotations replaced.
const ProseLegend:&str = "legend:";

/// The sentence a page must carry before it may keep a fenced Mermaid block.
const Unexportable:&str = "cannot be exported";

/// `Documentation/Diagram/`, from this crate's own manifest directory.
fn root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../../Documentation/Diagram")
		.canonicalize()
		.expect("Documentation/Diagram is reachable from Crate/Agentic")
}

/// Every file under a directory, in path order.
fn files(Directory:&Path) -> Vec<PathBuf> {
	let mut Found = Vec::new();
	let Ok(Entries) = fs::read_dir(Directory) else {
		return Found;
	};
	for Entry in Entries.flatten() {
		let Path = Entry.path();
		if Path.is_dir() {
			Found.extend(files(&Path));
		} else {
			Found.push(Path);
		}
	}
	Found.sort();
	Found
}

/// The path with its extension swapped, which is the sibling a file must have.
fn sibling(Path:&Path, Extension:&str) -> PathBuf { Path.with_extension(Extension) }

/// Whether the text holds `Word` as a whole identifier.
fn has_word(Text:&str, Word:&str) -> bool {
	Text.match_indices(Word).any(|(At, _)| {
		let Before = Text[..At].chars().next_back();
		let After = Text[At + Word.len()..].chars().next();
		let Boundary = |One:Option<char>| One.is_none_or(|One| !One.is_alphanumeric() && One != '_');
		Boundary(Before) && Boundary(After)
	})
}

/// Whether a line reads `note left`, `note right` or `note over`.
fn is_note(Line:&str) -> bool {
	let Trimmed = Line.trim();
	let Some(Rest) = Trimmed.strip_prefix("note") else {
		return false;
	};
	let Rest = Rest.trim_start();
	["left", "right", "over"].iter().any(|Side| {
		Rest.strip_prefix(Side)
			.is_some_and(|After| After.is_empty() || !After.starts_with(|One:char| One.is_alphanumeric()))
	})
}

/// Whether a line opens a `subgraph Legend`, which is a prose legend with a box
/// around it.
fn is_legend_subgraph(Line:&str) -> bool {
	let Trimmed = Line.trim().to_lowercase();
	let Some(Rest) = Trimmed.strip_prefix("subgraph") else {
		return false;
	};
	Rest.trim_start().starts_with("legend")
}

/// The frontmatter of a diagram, or nothing when it does not open with one.
fn frontmatter(Text:&str) -> Option<&str> {
	let Rest = Text.strip_prefix("---\n").or_else(|| Text.strip_prefix("---\r\n"))?;
	let End = Rest.find("\n---")?;
	Some(&Rest[..End])
}

/// One diagram's problems, as sentences.
fn diagram_problems(Name:&str, Path:&Path, Text:&str) -> Vec<String> {
	let mut Problems = Vec::new();
	let Some(Frontmatter) = frontmatter(Text) else {
		Problems.push(format!("{Name}: does not open with frontmatter"));
		return Problems;
	};
	let Lines:Vec<&str> = Frontmatter.lines().collect();
	if !Lines.iter().any(|Line| Line.trim() == "line9:") {
		Problems.push(format!("{Name}: frontmatter names no line9 block"));
	}
	let Themed = Lines.iter().any(|Line| {
		let Trimmed = Line.trim();
		let Some(Value) = Trimmed.strip_prefix("theme:") else {
			return false;
		};
		// The key must be indented - a top-level `theme:` is a different
		// document - and it must name something.
		Line.starts_with([' ', '\t']) && !Value.trim().is_empty()
	});
	if !Themed {
		Problems.push(format!("{Name}: frontmatter names no theme"));
	}
	if !Classes.iter().any(|Class| Text.contains(Class)) {
		Problems.push(format!("{Name}: carries no class annotation"));
	}
	if has_word(Text, "classDef") {
		Problems.push(format!("{Name}: declares a classDef; the theme is the CLI's"));
	}
	if Text.lines().any(is_note) {
		Problems.push(format!("{Name}: a note inside a drawing is a prose legend"));
	}
	if Text.lines().any(is_legend_subgraph) {
		Problems.push(format!("{Name}: a Legend subgraph is a prose legend"));
	}
	let Svg = sibling(Path, "svg");
	match fs::metadata(&Svg) {
		Ok(Meta) if Meta.len() <= MinimumSvgBytes => {
			Problems.push(format!("{Name}: its .svg is {} bytes, not rendered", Meta.len()));
		},
		Ok(_) => {},
		Err(_) => Problems.push(format!("{Name}: no .svg sibling")),
	}
	Problems
}

/// One rendered file's problems: it must have a source, and it must be a
/// render.
fn render_problems(Name:&str, Path:&Path) -> Vec<String> {
	let mut Problems = Vec::new();
	if !sibling(Path, "mmd").exists() {
		Problems.push(format!("{Name}: an .svg with no .mmd beside it"));
		return Problems;
	}
	match fs::metadata(Path) {
		Ok(Meta) if Meta.len() <= MinimumSvgBytes => {
			Problems.push(format!("{Name}: {} bytes, which is not a render", Meta.len()));
		},
		_ => {},
	}
	Problems
}

/// One page's problems: no prose legend, and a fenced block only where it says
/// why it cannot be exported.
fn page_problems(Name:&str, Text:&str) -> Vec<String> {
	let mut Problems = Vec::new();
	if Text.contains(ProseLegend) {
		Problems.push(format!("{Name}: carries a prose legend"));
	}
	if Text.contains("```mermaid") && !Text.contains(Unexportable) {
		Problems.push(format!("{Name}: keeps a fenced block without saying why it cannot be exported"));
	}
	Problems
}

/// The whole set, and the counts the sentence reports.
fn check() -> (usize, usize, Vec<String>) {
	let Root = root();
	let mut Problems = Vec::new();
	let mut Diagrams = 0usize;
	let mut Pages = 0usize;
	for Path in files(&Root) {
		let Name = Path.strip_prefix(&Root).unwrap_or(&Path).to_string_lossy().replace('\\', "/");
		match Path.extension().and_then(|Extension| Extension.to_str()) {
			Some("svg") => Problems.extend(render_problems(&Name, &Path)),
			Some("mmd") => {
				Diagrams += 1;
				let Text = fs::read_to_string(&Path).expect("a diagram is readable");
				Problems.extend(diagram_problems(&Name, &Path, &Text));
			},
			Some("md") => {
				Pages += 1;
				let Text = fs::read_to_string(&Path).expect("a page is readable");
				Problems.extend(page_problems(&Name, &Text));
			},
			_ => {},
		}
	}
	if Diagrams == 0 {
		Problems.push("Documentation/Diagram holds no .mmd at all".to_string());
	}
	(Diagrams, Pages, Problems)
}

/// Every diagram is typed, annotated, rendered, and free of a prose legend.
#[test]
fn every_diagram_is_typed_and_rendered() {
	let (Diagrams, Pages, Problems) = check();
	assert!(
		Problems.is_empty(),
		"{} problem(s) across {Diagrams} diagram(s) and {Pages} page(s):\n{}",
		Problems.len(),
		Problems.join("\n")
	);
}

/// The rules, on text this file owns.
///
/// The set above is green today, and a green set is also what a check that has
/// stopped reading anything reports. These cases hold the four refusals to a
/// malformed page, so a rule that is deleted or inverted fails here rather than
/// passing quietly over a tree that happens to be correct.
mod refusals {
	use std::path::Path;

	use super::{diagram_problems, page_problems, render_problems};

	#[test]
	fn a_diagram_without_frontmatter_is_refused() {
		let Problems = diagram_problems("Probe.mmd", Path::new("/nonexistent/Probe.mmd"), "graph TD\n");
		assert!(Problems.iter().any(|One| One.contains("frontmatter")), "{Problems:?}");
	}

	#[test]
	fn a_diagram_without_a_theme_or_an_annotation_is_refused() {
		let Text = "---\nline9:\n	layout: elk\n---\n\ngraph TD\n	A --> B\n";
		let Problems = diagram_problems("Probe.mmd", Path::new("/nonexistent/Probe.mmd"), Text);
		assert!(Problems.iter().any(|One| One.contains("no theme")), "{Problems:?}");
		assert!(Problems.iter().any(|One| One.contains("no class annotation")), "{Problems:?}");
		assert!(Problems.iter().any(|One| One.contains("no .svg sibling")), "{Problems:?}");
	}

	#[test]
	fn a_prose_legend_inside_a_drawing_is_refused() {
		for Text in [
			"---\nline9:\n	theme: Shared\n---\n\ngraph TD\n	A[\"x\"]:::host\n	note left of A: a legend\n",
			"---\nline9:\n	theme: Shared\n---\n\ngraph TD\n	A[\"x\"]:::host\n	subgraph Legend\n	end\n",
			"---\nline9:\n	theme: Shared\n---\n\ngraph TD\n	A[\"x\"]:::host\n	classDef host fill:#fff\n",
		] {
			let Problems = diagram_problems("Probe.mmd", Path::new("/nonexistent/Probe.mmd"), Text);
			assert!(
				Problems
					.iter()
					.any(|One| One.contains("prose legend") || One.contains("classDef")),
				"{Text:?} answered {Problems:?}"
			);
		}
	}

	#[test]
	fn a_page_keeping_a_fenced_block_without_a_reason_is_refused() {
		let Problems = page_problems("Page.md", "```mermaid\ngraph TD\n```\n");
		assert!(Problems.iter().any(|One| One.contains("cannot be exported")), "{Problems:?}");
		assert!(page_problems("Page.md", "```mermaid\ngraph TD\n```\n\ncannot be exported\n").is_empty());
	}

	#[test]
	fn a_render_with_no_source_is_refused() {
		let Problems = render_problems("Probe.svg", Path::new("/nonexistent/Probe.svg"));
		assert!(Problems.iter().any(|One| One.contains("no .mmd")), "{Problems:?}");
	}
}
