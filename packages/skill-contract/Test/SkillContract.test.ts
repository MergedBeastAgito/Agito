/**
 * The contract test for `agentic.skill.validate`.
 *
 * Every corpus here is built by the test in a fresh temporary directory and
 * removed afterwards, so what each rule is about is visible in this file rather
 * than hidden in a committed fixture tree that drifts. Five skills, one per rule
 * and one valid:
 *
 *   Alpha   - valid: a description, a reference, and an index that names it
 *   Beta    - an unlisted reference: `Reference/Unlisted.md` is not in the index
 *   Gamma   - unparsable frontmatter: a line that is not a key and a value
 *   Delta   - a descriptionless frontmatter, which is the same rule as Gamma
 *   Epsilon - uncited: valid, but the corpus root's index does not name it
 *
 *   node --test Test/*.test.ts
 */
import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, symlink, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { after, test } from "node:test";

import { Create, Rules, type Finding, type Skill } from "../src/index.ts";

const Roots: string[] = [];

/** A fresh corpus root, removed when the file's tests are done. */
const Scratch = async (): Promise<string> => {
	const root = await mkdtemp(join(tmpdir(), "skill-contract-"));
	Roots.push(root);
	return root;
};

after(async () => {
	for (const root of Roots) await rm(root, { recursive: true, force: true });
});

const Write = async (path: string, text: string): Promise<void> => {
	await mkdir(dirname(path), { recursive: true });
	await writeFile(path, text, "utf8");
};

/** One skill directory in a corpus under construction. */
type Fixture = {
	/** The directory under the corpus root. The skill's name is this string. */
	name: string;
	/** The text between the frontmatter fences of `SKILL.md`. */
	frontmatter: string;
	/** Files under `Reference/`, by file name. */
	references?: Record<string, string>;
	/** `Reference/Index.md`. Absent means the skill has no reference index. */
	referenceIndex?: string;
	/** Whether the corpus root's `Index.md` names this skill. Defaults to true. */
	cited?: boolean;
};

/** Write a corpus and its root index, and answer the root's path. */
const Corpus = async (fixtures: Fixture[]): Promise<string> => {
	const root = await Scratch();
	const cited: string[] = [];
	for (const fixture of fixtures) {
		const directory = join(root, fixture.name);
		await Write(
			join(directory, "SKILL.md"),
			`---\n${fixture.frontmatter}\n---\n\n# ${fixture.name}\n\nA body.\n`,
		);
		for (const [file, text] of Object.entries(fixture.references ?? {})) {
			await Write(join(directory, "Reference", file), text);
		}
		if (fixture.referenceIndex !== undefined) {
			await Write(join(directory, "Reference", "Index.md"), fixture.referenceIndex);
		}
		if (fixture.cited !== false) cited.push(fixture.name);
	}
	await Write(
		join(root, "Index.md"),
		cited.map((name) => `- [${name}/](${name}/)`).join("\n") + "\n",
	);
	return root;
};

const ValidSkill = (): Fixture => ({
	name: "Alpha",
	frontmatter: "name: alpha\ndescription: One honest skill, with its reference listed.",
	references: { "Notes.md": "# Notes\n" },
	referenceIndex: "- [Notes.md](Notes.md)\n",
});

const UnlistedReferenceSkill = (): Fixture => ({
	name: "Beta",
	frontmatter: "name: beta\ndescription: A skill whose index forgets one of its files.",
	references: { "Listed.md": "# Listed\n", "Unlisted.md": "# Unlisted\n" },
	referenceIndex: "- [Listed.md](Listed.md)\n",
});

const UnparsableFrontmatterSkill = (): Fixture => ({
	name: "Gamma",
	frontmatter: "name: gamma\nthis line is neither a key nor a value",
});

const DescriptionlessSkill = (): Fixture => ({
	name: "Delta",
	frontmatter: "name: delta",
});

const UncitedSkill = (): Fixture => ({
	name: "Epsilon",
	frontmatter: "name: epsilon\ndescription: A skill the corpus index forgets.",
	cited: false,
});

const CorpusOfEveryRule = (): Fixture[] => [
	ValidSkill(),
	UnlistedReferenceSkill(),
	UnparsableFrontmatterSkill(),
	DescriptionlessSkill(),
	UncitedSkill(),
];

/** The discovered record for one named skill. */
const Discovered = async (root: string, name: string): Promise<Skill> => {
	const skills = await Create().discover(root);
	const skill = skills.find((candidate) => candidate.name === name);
	assert.ok(skill, `no skill named ${name} was discovered`);
	return skill;
};

const RulesOf = (findings: Finding[]): string[] => findings.map((finding) => finding.rule);

test("discover returns every skill directory, ordered by name, whatever order they were created in", async () => {
	const root = await Corpus([
		{
			name: "Zulu",
			frontmatter: "description: Created first, sorted last.",
		},
		{
			name: "Alpha",
			frontmatter: "description: Created second, sorted first.",
		},
		{
			name: "Mike",
			frontmatter: "description: Created third, sorted second.",
		},
	]);
	await Write(join(root, "README.md"), "a file at the root is not a skill\n");
	const skills = await Create().discover(root);
	assert.deepEqual(
		skills.map((skill) => skill.name),
		["Alpha", "Mike", "Zulu"],
	);
});

test("discover is deterministic: two runs over the same tree agree", async () => {
	const root = await Corpus(CorpusOfEveryRule());
	const contract = Create();
	const first = await contract.discover(root);
	const second = await contract.discover(root);
	assert.deepEqual(second, first);
	assert.deepEqual(
		first.map((skill) => skill.name),
		["Alpha", "Beta", "Delta", "Epsilon", "Gamma"],
	);
	assert.deepEqual(first[0]?.references, ["Notes.md"]);
});

test("a symlinked skill directory is discovered like a real one", async () => {
	const root = await Corpus([]);
	const elsewhere = await Scratch();
	await Write(
		join(elsewhere, "Linked", "SKILL.md"),
		"---\ndescription: Linked in from outside the corpus.\n---\n\nA body.\n",
	);
	await symlink(join(elsewhere, "Linked"), join(root, "Linked"));
	const skills = await Create().discover(root);
	assert.deepEqual(
		skills.map((skill) => skill.name),
		["Linked"],
	);
	assert.equal(skills[0]?.description, "Linked in from outside the corpus.");
});

test("a valid skill has no findings", async () => {
	const root = await Corpus([ValidSkill()]);
	assert.deepEqual(await Create().validate(await Discovered(root, "Alpha")), []);
});

test("a reference file the index does not name is one unlisted-reference finding", async () => {
	const root = await Corpus([UnlistedReferenceSkill()]);
	const findings = await Create().validate(await Discovered(root, "Beta"));
	assert.deepEqual(RulesOf(findings), ["unlisted-reference"]);
	assert.equal(findings[0]?.skill, "Beta");
	assert.match(findings[0]?.detail ?? "", /Reference\/Unlisted\.md/);
});

test("a reference tree without an index reports each file as unlisted", async () => {
	const root = await Corpus([
		{
			name: "Loose",
			frontmatter: "name: loose\ndescription: A skill with files and no index.",
			references: { "One.md": "# One\n", "Two.md": "# Two\n" },
		},
	]);
	const findings = await Create().validate(await Discovered(root, "Loose"));
	assert.deepEqual(RulesOf(findings), ["unlisted-reference", "unlisted-reference"]);
	assert.match(findings[0]?.detail ?? "", /there is no Reference\/Index\.md/);
});

test("an index that names a longer file does not name the shorter one", async () => {
	const root = await Corpus([
		{
			name: "Prefix",
			frontmatter: "name: prefix\ndescription: An index that names the backup.",
			references: { "Notes.md": "# Notes\n" },
			referenceIndex: "- [Notes.md.bak](Notes.md.bak)\n",
		},
	]);
	const findings = await Create().validate(await Discovered(root, "Prefix"));
	assert.deepEqual(RulesOf(findings), ["unlisted-reference"]);
	assert.match(findings[0]?.detail ?? "", /Reference\/Notes\.md/);
});

test("a skill directory without a SKILL.md is unparsable and uncited, not an exception", async () => {
	const root = await Corpus([]);
	await mkdir(join(root, "Bare"), { recursive: true });
	const skill = await Discovered(root, "Bare");
	const contract = Create();
	await assert.doesNotReject(() => contract.validate(skill));
	assert.deepEqual(RulesOf(await contract.validate(skill)), ["unparsable", "uncited"]);
});

test("unparsable frontmatter is a finding, not a rejection", async () => {
	const root = await Corpus([UnparsableFrontmatterSkill()]);
	const skill = await Discovered(root, "Gamma");
	const contract = Create();
	await assert.doesNotReject(() => contract.validate(skill));
	const findings = await contract.validate(skill);
	assert.deepEqual(RulesOf(findings), ["unparsable"]);
	assert.match(findings[0]?.detail ?? "", /frontmatter/);
});

test("a frontmatter without a description is unparsable", async () => {
	const root = await Corpus([DescriptionlessSkill()]);
	const findings = await Create().validate(await Discovered(root, "Delta"));
	assert.deepEqual(RulesOf(findings), ["unparsable"]);
	assert.match(findings[0]?.detail ?? "", /description/);
});

test("a frontmatter name that disagrees with the directory is unparsable", async () => {
	const root = await Corpus([
		{
			name: "Named",
			frontmatter: "name: somewhere-else\ndescription: A rename stopped halfway.",
		},
	]);
	const findings = await Create().validate(await Discovered(root, "Named"));
	assert.deepEqual(RulesOf(findings), ["unparsable"]);
	assert.match(findings[0]?.detail ?? "", /somewhere-else/);
	assert.match(findings[0]?.detail ?? "", /Named/);
});

test("a corpus root that names its skills in AGENTS.md is read as well as one that uses Index.md", async () => {
	const root = await Corpus([ValidSkill()]);
	await rm(join(root, "Index.md"));
	await Write(join(root, "AGENTS.md"), "- [Alpha/](Alpha/)\n");
	assert.deepEqual(await Create().validate(await Discovered(root, "Alpha")), []);
});

test("a skill the corpus index does not name is uncited", async () => {
	const root = await Corpus([UncitedSkill()]);
	const findings = await Create().validate(await Discovered(root, "Epsilon"));
	assert.deepEqual(RulesOf(findings), ["uncited"]);
	assert.match(findings[0]?.detail ?? "", /Epsilon/);
});

test("every finding uses one of the three rules the manifest declares", async () => {
	const manifest = JSON.parse(
		await readFile(new URL("../module.json", import.meta.url), "utf8"),
	) as { provides: { errors: string[] } };
	const declared = manifest.provides.errors;
	assert.deepEqual([...Rules].sort(), [...declared].sort());
	const root = await Corpus(CorpusOfEveryRule());
	const contract = Create();
	const findings: Finding[] = [];
	for (const skill of await contract.discover(root)) {
		findings.push(...(await contract.validate(skill)));
	}
	assert.ok(findings.length >= 4);
	for (const finding of findings) {
		assert.ok(
			declared.includes(finding.rule),
			`undeclared rule ${finding.rule} in ${JSON.stringify(finding)}`,
		);
	}
});

test("validate is stateless: the same skill answers the same findings twice", async () => {
	const root = await Corpus([UnlistedReferenceSkill()]);
	const skill = await Discovered(root, "Beta");
	const contract = Create();
	assert.deepEqual(await contract.validate(skill), await contract.validate(skill));
});

test("validate re-reads the directory, so a file added after the first call is reported by the second", async () => {
	const root = await Corpus([ValidSkill()]);
	const skill = await Discovered(root, "Alpha");
	const contract = Create();
	assert.deepEqual(await contract.validate(skill), []);
	await Write(join(skill.directory, "Reference", "Later.md"), "# Later\n");
	const findings = await contract.validate(skill);
	assert.deepEqual(RulesOf(findings), ["unlisted-reference"]);
	assert.match(findings[0]?.detail ?? "", /Later\.md/);
});

test("a skill with no Reference tree is valid", async () => {
	const root = await Corpus([
		{
			name: "Solo",
			frontmatter: "name: solo\ndescription: A skill with no worked instance yet.",
		},
	]);
	assert.deepEqual(await Create().validate(await Discovered(root, "Solo")), []);
});

test("a folded description is read as the description", async () => {
	const root = await Corpus([
		{
			name: "Folded",
			frontmatter: "name: folded\ndescription: >\n  A description that\n  spans two lines.",
		},
	]);
	const skill = await Discovered(root, "Folded");
	assert.equal(skill.description, "A description that spans two lines.");
	assert.deepEqual(await Create().validate(skill), []);
});

test("an unreadable root is an empty corpus, not a rejection", async () => {
	const contract = Create();
	const root = join(await Scratch(), "there-is-no-corpus-here");
	await assert.doesNotReject(() => contract.discover(root));
	assert.deepEqual(await contract.discover(root), []);
	assert.match(contract.digest([]), /^sha256:[0-9a-f]{64}$/);
});

test("the digest ignores modification times", async () => {
	const root = await Corpus([ValidSkill()]);
	const contract = Create();
	const before = contract.digest(await contract.discover(root));
	const later = new Date(Date.now() + 86_400_000);
	await utimes(join(root, "Alpha", "SKILL.md"), later, later);
	await utimes(join(root, "Alpha", "Reference", "Notes.md"), later, later);
	assert.equal(contract.digest(await contract.discover(root)), before);
});

test("the digest changes when a name, a description, a reference or the order changes", async () => {
	const root = await Corpus([ValidSkill(), DescriptionlessSkill()]);
	const contract = Create();
	const skills = await contract.discover(root);
	const first = skills[0] as Skill;
	const second = skills[1] as Skill;
	const digest = contract.digest(skills);
	assert.notEqual(contract.digest([second, first]), digest);
	assert.notEqual(contract.digest([{ ...first, name: "Omega" }, second]), digest);
	assert.notEqual(
		contract.digest([{ ...first, description: "Something else." }, second]),
		digest,
	);
	assert.notEqual(
		contract.digest([{ ...first, references: [...first.references, "Extra.md"] }, second]),
		digest,
	);
	assert.equal(contract.digest([{ ...first, directory: "/somewhere/else" }, second]), digest);
});

test("the digest of the valid skill is the recorded one", async () => {
	const root = await Corpus([ValidSkill()]);
	const skill = await Discovered(root, "Alpha");
	assert.equal(
		Create().digest([skill]),
		"sha256:86b8b052e5214defc8a0940faccaba3b4436a5d5545afabcc08ec5ad7cb8b4f9",
	);
});
