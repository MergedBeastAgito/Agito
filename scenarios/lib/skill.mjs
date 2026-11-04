/**
 * The TypeScript side of the differential run for `agentic.skill.validate`.
 *
 * This capability reads a real filesystem, so a scenario does not describe a
 * fake: it describes a *corpus* - a tree of files - and the harness builds that
 * tree in a fresh temporary directory before the port ever runs. The Rust half
 * (`Crate/SkillContract/Test/Parity.rs`) builds the same tree, byte for
 * byte, from the same description, so what the two ports are handed differs
 * only in the root path - and the root path is deliberately outside every
 * observation, because `directory` is excluded from the digest and no effect
 * carries a path.
 *
 * The tree, as `fixture` describes it:
 *
 *   corpus[]        one skill directory each: `name` is the directory name,
 *                   `frontmatter` is the text between the fences of `SKILL.md`,
 *                   `references` are files under `Reference/`, `referenceIndex`
 *                   is `Reference/Index.md`, `cited` (default true) decides
 *                   whether the corpus root's index names it, `noBody` leaves
 *                   the directory without a `SKILL.md`.
 *   symlink         one more skill, written *outside* the corpus root and
 *                   linked into it by name - the only way to describe a skill
 *                   that is reached through a link.
 *   corpusIndex     `Index.md`, `AGENTS.md` or null. Its text is generated from
 *                   the `cited` flags, so both sides write the same bytes.
 *   rootFiles       files at the corpus root, which are not skills.
 *   absentRoot      the corpus root does not exist at all.
 *   touchAfterFirstValidate  a path under the first validated skill, written
 *                   after that call returns - the file a re-reading validate
 *                   must see.
 *
 * The steps, and nothing else:
 *
 *   discover              run `discover`; answer `{kind: "names"}`.
 *   validate <skill>      discover, then validate that record; answer
 *                         `{kind: "findings"}`.
 *   validateDiscovered    validate the record the last `discover` answered, so
 *                         no second discover happens; answer `{kind:"findings"}`.
 *   digest                digest the last `discover` answered, a literal
 *                         `skills` list, or that list with one `mutate` applied
 *                         (`{reverse: true}` or `{at, ...fields}`); answer
 *                         `{kind: "digest"}`.
 *   touch                 push every file's modification time a day forward.
 *
 * The effects are what the port reported, in order: `discovered:<name>` for
 * every skill a `discover` answered, and `finding:<rule>:<skill>` for every
 * finding a `validate` reported. `digest` and `touch` have no effects. A port
 * that answered the same findings in a different order, or that discovered
 * twice where the scenario says once, fails on the effect list alone.
 */
import { mkdir, mkdtemp, readdir, rm, symlink, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";

import { Create } from "../../Package/skill-contract/Source/index.ts";

/** One `SKILL.md`, as the contract suite writes it. */
const Body = (name, frontmatter) => `---\n${frontmatter}\n---\n\n# ${name}\n\nA body.\n`;

/** The corpus root's index, from the skills that are cited. */
const Index = (names) => names.map((name) => `- [${name}/](${name}/)`).join("\n") + "\n";

const Write = async (path, text) => {
	await mkdir(dirname(path), { recursive: true });
	await writeFile(path, text, "utf8");
};

/** The file a `touchAfterFirstValidate` hook writes, and its one line of text. */
const Stem = (path) => basename(path).replace(/\.[^.]*$/, "");

/** Every skill a fixture describes: the corpus, then the one linked in. */
const SkillsOf = (fixture) => [
	...(fixture.corpus ?? []),
	...(fixture.symlink === undefined || fixture.symlink === null
		? []
		: [{ ...fixture.symlink, external: true }]),
];

/**
 * Build the corpus a fixture describes, and answer the root.
 *
 * The root sits under a fresh `mkdtemp`, one directory in from it, so the
 * symlinked skill has somewhere to live that is genuinely outside the corpus -
 * a link inside the corpus would prove nothing.
 */
export const Materialise = async (fixture) => {
	const base = await mkdtemp(join(tmpdir(), "skill-parity-"));
	const root = join(base, "corpus");
	const outside = join(base, "outside");
	if (fixture.absentRoot === true) return { base, root };
	await mkdir(root, { recursive: true });
	const cited = [];
	for (const skill of SkillsOf(fixture)) {
		const home = skill.external === true ? join(outside, skill.name) : join(root, skill.name);
		await mkdir(home, { recursive: true });
		if (skill.noBody !== true)
			await Write(join(home, "SKILL.md"), Body(skill.name, skill.frontmatter ?? ""));
		for (const [file, text] of Object.entries(skill.references ?? {}))
			await Write(join(home, "Reference", file), text);
		if (skill.referenceIndex !== undefined && skill.referenceIndex !== null)
			await Write(join(home, "Reference", "Index.md"), skill.referenceIndex);
		if (skill.external === true) await symlink(home, join(root, skill.name));
		if (skill.cited !== false) cited.push(skill.name);
	}
	for (const [file, text] of Object.entries(fixture.rootFiles ?? {}))
		await Write(join(root, file), text);
	if (fixture.corpusIndex !== undefined && fixture.corpusIndex !== null)
		await Write(join(root, fixture.corpusIndex), Index(cited));
	return { base, root };
};

/** A day forward, on every file under `path`. */
const Bump = async (path) => {
	const entries = await readdir(path, { withFileTypes: true });
	for (const entry of entries) {
		const child = join(path, entry.name);
		if (entry.isDirectory()) await Bump(child);
		else if (entry.isFile()) {
			const later = new Date(Date.now() + 86_400_000);
			await utimes(child, later, later);
		}
	}
};

/** One record, with the four fields the port promises and nothing else. */
const Record = (value) => ({
	name: String(value.name ?? ""),
	directory: String(value.directory ?? ""),
	description: String(value.description ?? ""),
	references: [...(value.references ?? [])],
});

/** The records a `digest` step hands the port. */
const Digested = (one, held) => {
	if (one.skills !== undefined) return one.skills.map(Record);
	let records = held.map(Record);
	const mutate = one.mutate ?? {};
	if (mutate.reverse === true) records = [...records].reverse();
	if (mutate.at !== undefined) {
		const { at, ...fields } = mutate;
		records = records.map((record, index) =>
			index === at ? { ...record, ...fields } : record,
		);
	}
	return records;
};

/** Run one skill-contract case and answer what it did. */
export const runSkill = async (entry) => {
	const fixture = entry.fixture;
	const effects = [];
	const { base, root } = await Materialise(fixture);
	const contract = Create();
	let held = [];
	let touched = false;
	let result = null;
	try {
		for (const [index, one] of entry.steps.entries()) {
			if (one.op === "discover") {
				held = await contract.discover(root);
				for (const skill of held) effects.push(`discovered:${skill.name}`);
				result = { kind: "names", names: held.map((skill) => skill.name) };
				continue;
			}
			if (one.op === "touch") {
				await Bump(root);
				continue;
			}
			if (one.op === "digest") {
				result = { kind: "digest", value: contract.digest(Digested(one, held)) };
				continue;
			}
			if (one.op === "validate" || one.op === "validateDiscovered") {
				let record;
				if (one.op === "validate") {
					const discovered = await contract.discover(root);
					for (const skill of discovered) effects.push(`discovered:${skill.name}`);
					record = discovered.find((skill) => skill.name === one.skill);
				} else {
					record = held.find((skill) => skill.name === one.skill);
				}
				if (record === undefined)
					throw new Error(`${entry.id} step ${index}: no record for ${one.skill}`);
				const findings = await contract.validate(record);
				for (const finding of findings)
					effects.push(`finding:${finding.rule}:${finding.skill}`);
				result = { kind: "findings", rules: findings.map((finding) => finding.rule) };
				if (!touched && typeof fixture.touchAfterFirstValidate === "string") {
					touched = true;
					await Write(
						join(record.directory, fixture.touchAfterFirstValidate),
						`# ${Stem(fixture.touchAfterFirstValidate)}\n`,
					);
				}
				continue;
			}
			throw new Error(`${entry.id}: unknown skill step ${JSON.stringify(one.op)}`);
		}
	} finally {
		await rm(base, { recursive: true, force: true });
	}
	return { result, effects };
};
