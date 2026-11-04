//! `agentic.skill.validate`, exercised against corpora this test builds on
//! disk.
//!
//! No network, no browser and no fixture files checked in: a corpus is a
//! temporary directory the test writes, so a test can add a file between two
//! calls and prove that nothing was cached.

use std::{
	fs,
	path::{Path, PathBuf},
};

use AgenticRuntime::BlockOn;
use AgenticSkillContract::{Create, Finding, Rule, Rules, Skill, SkillContract};

/// A fresh, empty corpus root.
fn Root(Tag:&str) -> PathBuf {
	let Base = Path::new(env!("CARGO_TARGET_TMPDIR")).join(Tag);
	let _ = fs::remove_dir_all(&Base);
	fs::create_dir_all(&Base).expect("the corpus root is writable");
	Base
}

/// A skill directory carrying one `SKILL.md`.
fn SkillAt(Root:&Path, Name:&str, Frontmatter:&str) -> PathBuf {
	let Directory = Root.join(Name);
	fs::create_dir_all(&Directory).expect("the skill directory is writable");
	fs::write(Directory.join("SKILL.md"), format!("---\n{Frontmatter}\n---\n\n# {Name}\n"))
		.expect("SKILL.md is writable");
	Directory
}

/// One file under a skill's `Reference/`.
fn ReferenceAt(Directory:&Path, Name:&str, Body:&str) {
	let References = Directory.join("Reference");
	fs::create_dir_all(&References).expect("Reference/ is writable");
	fs::write(References.join(Name), Body).expect("the reference is writable");
}

/// A skill's `Reference/Index.md`.
fn ReferenceIndexAt(Directory:&Path, Body:&str) { ReferenceAt(Directory, "Index.md", Body); }

/// The corpus root's index.
fn CorpusIndexAt(Root:&Path, Name:&str, Body:&str) {
	fs::write(Root.join(Name), Body).expect("the corpus index is writable");
}

fn Discovered(Root:&Path, Name:&str) -> Skill {
	let Skills = BlockOn(Create().Discover(&Root.to_string_lossy()));
	Skills
		.into_iter()
		.find(|Candidate| Candidate.Name == Name)
		.unwrap_or_else(|| panic!("no skill named {Name} was discovered"))
}

fn Findings(Skill:&Skill) -> Vec<Finding> { BlockOn(Create().Validate(Skill)) }

fn RulesOf(Findings:&[Finding]) -> Vec<&str> { Findings.iter().map(|Finding| Finding.Rule.as_str()).collect() }

/// The fixture the recorded digest belongs to.
fn Valid(Root:&Path) -> Skill {
	let Directory = SkillAt(
		Root,
		"Alpha",
		"name: alpha\ndescription: One honest skill, with its reference listed.",
	);
	ReferenceAt(&Directory, "Notes.md", "# Notes\n");
	ReferenceIndexAt(&Directory, "- [Notes.md](Notes.md)\n");
	CorpusIndexAt(Root, "Index.md", "- [Alpha](Alpha/SKILL.md)\n");
	Discovered(Root, "Alpha")
}

#[test]
fn the_port_declares_exactly_the_errors_the_manifest_promises() {
	let Declared:Vec<&str> = Rules.into_iter().map(Rule::Name).collect();
	assert_eq!(Declared, vec!["unparsable", "unlisted-reference", "uncited"]);
	let Manifest = include_str!("../module.json");
	for Name in Declared {
		assert!(Manifest.contains(&format!("\"{Name}\"")), "{Name} is not in module.json");
	}
}

#[test]
fn discover_returns_every_skill_directory_ordered_by_name() {
	let Root = Root("order");
	SkillAt(&Root, "Zulu", "description: Created first, sorted last.");
	SkillAt(&Root, "Alpha", "description: Created second, sorted first.");
	let Names:Vec<String> = BlockOn(Create().Discover(&Root.to_string_lossy()))
		.into_iter()
		.map(|Skill| Skill.Name)
		.collect();
	assert_eq!(Names, vec!["Alpha", "Zulu"]);
}

#[test]
fn discover_is_deterministic_across_two_runs() {
	let Root = Root("deterministic");
	SkillAt(&Root, "Beta", "description: One.");
	SkillAt(&Root, "Alpha", "description: Two.");
	SkillAt(&Root, "Gamma", "description: Three.");
	let First = BlockOn(Create().Discover(&Root.to_string_lossy()));
	let Second = BlockOn(Create().Discover(&Root.to_string_lossy()));
	assert_eq!(First, Second);
}

#[cfg(unix)]
#[test]
fn a_symlinked_skill_directory_is_discovered_like_a_real_one() {
	let Elsewhere = Root("symlink-real");
	SkillAt(&Elsewhere, "Linked", "description: Reached through a link.");
	let Corpus = Root("symlink");
	std::os::unix::fs::symlink(Elsewhere.join("Linked"), Corpus.join("Linked")).expect("the link is created");
	let Names:Vec<String> = BlockOn(Create().Discover(&Corpus.to_string_lossy()))
		.into_iter()
		.map(|Skill| Skill.Name)
		.collect();
	assert_eq!(Names, vec!["Linked"]);
}

#[test]
fn a_valid_skill_has_no_findings() {
	let Root = Root("valid");
	let Skill = Valid(&Root);
	assert_eq!(Findings(&Skill), vec![]);
	assert_eq!(Skill.Description, "One honest skill, with its reference listed.");
	assert_eq!(Skill.References, vec!["Notes.md"]);
}

#[test]
fn a_reference_file_the_index_does_not_name_is_one_unlisted_reference() {
	let Root = Root("unlisted");
	let Directory = SkillAt(&Root, "Beta", "name: beta\ndescription: One file is forgotten.");
	ReferenceAt(&Directory, "Listed.md", "# Listed\n");
	ReferenceAt(&Directory, "Unlisted.md", "# Unlisted\n");
	ReferenceIndexAt(&Directory, "- [Listed.md](Listed.md)\n");
	CorpusIndexAt(&Root, "Index.md", "- [Beta](Beta/SKILL.md)\n");
	let Found = Findings(&Discovered(&Root, "Beta"));
	assert_eq!(RulesOf(&Found), vec!["unlisted-reference"]);
	assert!(Found[0].Detail.contains("Unlisted.md"));
	assert_eq!(Found[0].Skill, "Beta");
}

#[test]
fn a_reference_tree_without_an_index_reports_each_file_as_unlisted() {
	let Root = Root("no-index");
	let Directory = SkillAt(&Root, "Gamma", "name: gamma\ndescription: No index at all.");
	ReferenceAt(&Directory, "One.md", "# One\n");
	ReferenceAt(&Directory, "Two.md", "# Two\n");
	CorpusIndexAt(&Root, "Index.md", "- [Gamma](Gamma/SKILL.md)\n");
	let Found = Findings(&Discovered(&Root, "Gamma"));
	assert_eq!(RulesOf(&Found), vec!["unlisted-reference", "unlisted-reference"]);
	assert!(Found[0].Detail.contains("there is no Reference/Index.md"));
}

#[test]
fn an_index_that_names_a_longer_file_does_not_name_the_shorter_one() {
	let Root = Root("substring");
	let Directory = SkillAt(&Root, "Delta", "name: delta\ndescription: One name hides another.");
	ReferenceAt(&Directory, "Notes.md", "# Notes\n");
	ReferenceAt(&Directory, "Notes.md.bak", "# Backup\n");
	ReferenceIndexAt(&Directory, "- [Notes.md.bak](Notes.md.bak)\n");
	CorpusIndexAt(&Root, "Index.md", "- [Delta](Delta/SKILL.md)\n");
	let Found = Findings(&Discovered(&Root, "Delta"));
	assert_eq!(RulesOf(&Found), vec!["unlisted-reference"]);
	assert!(Found[0].Detail.contains("Reference/Notes.md is not named"));
}

#[test]
fn a_skill_directory_without_a_body_is_unparsable_and_uncited() {
	let Root = Root("no-body");
	fs::create_dir_all(Root.join("Empty")).expect("the directory is writable");
	let Skill = Discovered(&Root, "Empty");
	let Found = Findings(&Skill);
	assert_eq!(RulesOf(&Found), vec!["unparsable", "uncited"]);
	assert!(Found[0].Detail.contains("carries no SKILL.md"));
}

#[test]
fn unparsable_frontmatter_is_a_finding_not_a_rejection() {
	let Root = Root("unparsable");
	SkillAt(&Root, "Epsilon", "name: epsilon\nthis line is neither a key nor a value");
	CorpusIndexAt(&Root, "Index.md", "- [Epsilon](Epsilon/SKILL.md)\n");
	let Found = Findings(&Discovered(&Root, "Epsilon"));
	assert_eq!(RulesOf(&Found), vec!["unparsable"]);
	assert!(Found[0].Detail.contains("is not a key: value pair"));
}

#[test]
fn a_frontmatter_without_a_description_is_unparsable() {
	let Root = Root("no-description");
	SkillAt(&Root, "Zeta", "name: zeta");
	CorpusIndexAt(&Root, "Index.md", "- [Zeta](Zeta/SKILL.md)\n");
	let Found = Findings(&Discovered(&Root, "Zeta"));
	assert_eq!(RulesOf(&Found), vec!["unparsable"]);
	assert!(Found[0].Detail.contains("states no description"));
}

#[test]
fn a_frontmatter_name_that_disagrees_with_the_directory_is_unparsable() {
	let Root = Root("renamed");
	SkillAt(&Root, "Eta", "name: theta\ndescription: A rename that stopped halfway.");
	CorpusIndexAt(&Root, "Index.md", "- [Eta](Eta/SKILL.md)\n");
	let Found = Findings(&Discovered(&Root, "Eta"));
	assert_eq!(RulesOf(&Found), vec!["unparsable"]);
	assert!(Found[0].Detail.contains("the frontmatter names \"theta\""));
}

#[test]
fn a_corpus_root_that_names_its_skills_in_agents_md_is_read_as_well() {
	let Root = Root("agents");
	SkillAt(&Root, "Iota", "name: iota\ndescription: Cited by the repository's own index.");
	CorpusIndexAt(&Root, "AGENTS.md", "- [Iota](Iota/SKILL.md)\n");
	assert_eq!(Findings(&Discovered(&Root, "Iota")), vec![]);
}

#[test]
fn a_skill_the_corpus_index_does_not_name_is_uncited() {
	let Root = Root("uncited");
	SkillAt(&Root, "Kappa", "name: kappa\ndescription: Nothing points at it.");
	CorpusIndexAt(&Root, "Index.md", "- [SomethingElse](SomethingElse/SKILL.md)\n");
	let Found = Findings(&Discovered(&Root, "Kappa"));
	assert_eq!(RulesOf(&Found), vec!["uncited"]);
	assert!(Found[0].Detail.contains("Index.md does not name Kappa/"));
}

#[test]
fn a_corpus_root_with_no_index_at_all_is_uncited() {
	let Root = Root("no-corpus-index");
	SkillAt(&Root, "Lambda", "name: lambda\ndescription: No index to cite it.");
	let Found = Findings(&Discovered(&Root, "Lambda"));
	assert_eq!(RulesOf(&Found), vec!["uncited"]);
	assert!(Found[0].Detail.contains("carries no Index.md or AGENTS.md"));
}

#[test]
fn every_finding_uses_one_of_the_three_rules_the_manifest_declares() {
	let Root = Root("vocabulary");
	SkillAt(&Root, "Mu", "");
	let Found = Findings(&Discovered(&Root, "Mu"));
	assert!(!Found.is_empty());
	for Finding in &Found {
		assert!(Finding.Code().is_some(), "{} is not one of the three rules", Finding.Rule);
	}
}

#[test]
fn validate_is_stateless_and_answers_the_same_findings_twice() {
	let Root = Root("stateless");
	let Skill = Valid(&Root);
	let First = Findings(&Skill);
	let Second = Findings(&Skill);
	assert_eq!(First, Second);
	assert_eq!(First, vec![]);
}

#[test]
fn validate_re_reads_the_directory_so_a_later_file_is_reported() {
	let Root = Root("re-read");
	let Skill = Valid(&Root);
	assert_eq!(Findings(&Skill), vec![]);
	ReferenceAt(Path::new(&Skill.Directory), "Added.md", "# Added later\n");
	let Found = Findings(&Skill);
	assert_eq!(RulesOf(&Found), vec!["unlisted-reference"]);
	assert!(Found[0].Detail.contains("Added.md"));
}

#[test]
fn a_skill_with_no_reference_tree_is_valid() {
	let Root = Root("no-references");
	SkillAt(&Root, "Nu", "name: nu\ndescription: Nothing but a body.");
	CorpusIndexAt(&Root, "Index.md", "- [Nu](Nu/SKILL.md)\n");
	let Skill = Discovered(&Root, "Nu");
	assert_eq!(Skill.References, Vec::<String>::new());
	assert_eq!(Findings(&Skill), vec![]);
}

#[test]
fn a_folded_description_is_read_as_the_description() {
	let Root = Root("folded");
	SkillAt(
		&Root,
		"Xi",
		"name: xi\ndescription: >\n  A description that spans\n  two lines.",
	);
	CorpusIndexAt(&Root, "Index.md", "- [Xi](Xi/SKILL.md)\n");
	let Skill = Discovered(&Root, "Xi");
	assert_eq!(Skill.Description, "A description that spans two lines.");
	assert_eq!(Findings(&Skill), vec![]);
}

#[test]
fn an_unreadable_root_is_an_empty_corpus_not_a_failure() {
	let Missing = Path::new(env!("CARGO_TARGET_TMPDIR")).join("there-is-no-corpus-here");
	let Skills = BlockOn(Create().Discover(&Missing.to_string_lossy()));
	assert_eq!(Skills, vec![]);
	let Digest = Create().Digest(&[]);
	assert_eq!(Digest.len(), "sha256:".len() + 64);
	assert!(Digest.starts_with("sha256:"));
}

#[test]
fn the_digest_ignores_modification_times_and_directories() {
	let Root = Root("digest-times");
	let Skill = Valid(&Root);
	let Before = Create().Digest(std::slice::from_ref(&Skill));
	let Moved = Skill { Directory:"/somewhere/else".to_string(), ..Skill.clone() };
	let After = Create().Digest(std::slice::from_ref(&Moved));
	assert_eq!(Before, After);
}

#[test]
fn the_digest_changes_when_a_name_a_description_a_reference_or_the_order_changes() {
	let Root = Root("digest-changes");
	let Skill = Valid(&Root);
	let Base = Create().Digest(std::slice::from_ref(&Skill));
	let Renamed = Skill { Name:"Alpha2".to_string(), ..Skill.clone() };
	assert_ne!(Base, Create().Digest(std::slice::from_ref(&Renamed)));
	let Described = Skill { Description:"Another sentence.".to_string(), ..Skill.clone() };
	assert_ne!(Base, Create().Digest(std::slice::from_ref(&Described)));
	let Referenced = Skill { References:vec!["Notes.md".to_string(), "Other.md".to_string()], ..Skill.clone() };
	assert_ne!(Base, Create().Digest(std::slice::from_ref(&Referenced)));
	let Other = Skill { Name:"Zulu".to_string(), ..Skill.clone() };
	assert_ne!(
		Create().Digest(&[Skill.clone(), Other.clone()]),
		Create().Digest(&[Other, Skill])
	);
}

#[test]
fn the_digest_of_the_valid_skill_is_the_recorded_one() {
	// The same value the TypeScript suite records, which is what proves the two
	// trees canonicalise a corpus identically rather than merely stably.
	let Root = Root("digest-recorded");
	let Skill = Valid(&Root);
	assert_eq!(
		Create().Digest(std::slice::from_ref(&Skill)),
		"sha256:86b8b052e5214defc8a0940faccaba3b4436a5d5545afabcc08ec5ad7cb8b4f9"
	);
}
