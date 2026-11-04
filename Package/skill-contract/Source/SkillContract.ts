/**
 * The implementation of the port in `Port.ts`: discover a corpus, report what a
 * skill breaks, digest the result.
 *
 * Everything here reads the filesystem and nothing here caches it. There is no
 * module state beyond the constants below, so two calls with the same arguments
 * answer the same findings and a call after the tree changed answers the new
 * ones.
 */
import { createHash } from "node:crypto";
import type { Dirent } from "node:fs";
import { readdir, readFile, stat } from "node:fs/promises";
import { dirname, join } from "node:path";

import type { Finding, Skill, SkillContract } from "./Port.ts";

const BodyFile = "SKILL.md";
const IndexFile = "Index.md";
const ReferenceDirectory = "Reference";

/**
 * The corpus root's index, by the two names a corpus actually gives it.
 *
 * `Index.md` is the port's own name, and the one it prefers when both exist.
 * `AGENTS.md` is the name a repository already gives the file its tools discover,
 * and a corpus that grows inside such a repository should not have to rename it
 * to be read - one file with two accepted names is cheaper than a corpus that is
 * uncited in its entirety. A skill's own `Reference/Index.md` has one name only:
 * there the card names the file.
 */
const CorpusIndexes = [IndexFile, "AGENTS.md"];

/**
 * Code-unit order, not `localeCompare`: a digest and a sweep must not change
 * meaning with the machine's locale.
 */
const Compare = (left: string, right: string): number => (left < right ? -1 : left > right ? 1 : 0);

/**
 * The entries of a directory, or none.
 *
 * A root that cannot be read is an empty corpus rather than an exception, because
 * a sweep that dies on one bad path cannot report the rest. A caller that must
 * tell a typo from an empty corpus stats the root itself - the port does not
 * pretend to know which of the two a missing directory was.
 */
const Entries = async (path: string): Promise<Dirent[]> => {
	try {
		return await readdir(path, { withFileTypes: true });
	} catch {
		return [];
	}
};

/** A file's text, or null. An unreadable file reads as absent for the reason above. */
const Text = async (path: string): Promise<string | null> => {
	try {
		return await readFile(path, "utf8");
	} catch {
		return null;
	}
};

/** Whether an entry is a directory, following a symlink one level. */
const IsDirectory = async (entry: Dirent, path: string): Promise<boolean> => {
	if (entry.isDirectory()) return true;
	if (!entry.isSymbolicLink()) return false;
	try {
		return (await stat(path)).isDirectory();
	} catch {
		return false;
	}
};

type Frontmatter = {
	data: Record<string, string> | null;
	error: string | null;
};

/**
 * A `|` or `>` block scalar with the block's own indentation removed.
 *
 * A folded scalar joins its lines into one, a literal one keeps the breaks. Both
 * drop blank lines, because a description is a sentence rather than a document
 * and the port reads exactly one field this way.
 */
const Scalar = (lines: string[], literal: boolean): string => {
	const indented = lines.filter((line) => line.trim() !== "");
	if (indented.length === 0) return "";
	const indent = Math.min(...indented.map((line) => /^[ 	]*/.exec(line)?.[0].length ?? 0));
	const stripped = lines.map((line) => line.slice(Math.min(indent, line.length)));
	return literal
		? stripped.join("\n").trim()
		: stripped
				.filter((line) => line.trim() !== "")
				.join(" ")
				.trim();
};

/**
 * The frontmatter block and the fields this port promises from it.
 *
 * A YAML parser would be a dependency, and the two fields a loader reads do not
 * justify one. What is read: top-level `key: value` pairs, a value quoted or
 * plain, and a `|` or `>` block scalar, which is how a long description is
 * usually written. What is not read: nested mappings and sequences. Those are
 * skipped rather than refused, because a skill may carry metadata this port does
 * not promise and a validator that rejects unfamiliar keys is a validator people
 * turn off.
 */
const ParseBlock = (lines: string[]): Frontmatter => {
	const data: Record<string, string> = {};
	let index = 0;
	while (index < lines.length) {
		const line = lines[index] ?? "";
		index += 1;
		const trimmed = line.trim();
		if (trimmed === "" || trimmed.startsWith("#")) continue;
		const pair = /^([A-Za-z0-9_.-]+):(.*)$/.exec(line);
		if (pair === null) {
			if (/^\s/.test(line)) continue;
			return {
				data: null,
				error: `${JSON.stringify(line)} is not a key: value pair`,
			};
		}
		const key = pair[1] ?? "";
		const value = (pair[2] ?? "").trim();
		if (/^[|>][+-]?$/.test(value)) {
			const collected: string[] = [];
			while (index < lines.length) {
				const next = lines[index] ?? "";
				if (next.trim() !== "" && !/^\s/.test(next)) break;
				collected.push(next);
				index += 1;
			}
			data[key] = Scalar(collected, value.startsWith("|"));
			continue;
		}
		data[key] = value.replace(/^(["'])(.*)\1$/, "$2");
	}
	return { data, error: null };
};

/** The YAML block at the top of a file. A file without one has nothing to read. */
const SplitFrontmatter = (text: string): Frontmatter => {
	const lines = text.replace(/\r\n/g, "\n").split("\n");
	if ((lines[0] ?? "").trim() !== "---") {
		return { data: null, error: "there is no frontmatter block" };
	}
	const end = lines.indexOf("---", 1);
	if (end === -1) {
		return { data: null, error: "the frontmatter block is never closed" };
	}
	return ParseBlock(lines.slice(1, end));
};

/**
 * Whether a text names a token, as a whole name.
 *
 * `includes` is not enough: `Notes.md` is a substring of `Notes.md.bak`, so an
 * index could appear to name a file it never names. The boundaries are the
 * characters a file or directory name is built from.
 */
const Names = (text: string, token: string): boolean => {
	const escaped = token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
	return new RegExp(`(^|[^\\w.-])${escaped}(?![\\w.-])`).test(text);
};

/** The file names directly under `Reference/`, excluding the index itself. */
const References = async (directory: string): Promise<string[]> => {
	const entries = await Entries(join(directory, ReferenceDirectory));
	return entries
		.filter((entry) => entry.isFile() || entry.isSymbolicLink())
		.map((entry) => entry.name)
		.filter((name) => name !== IndexFile)
		.sort(Compare);
};

/** The first index a corpus root carries, and the name it was found under. */
const CorpusIndex = async (directory: string): Promise<{ name: string; text: string } | null> => {
	for (const name of CorpusIndexes) {
		const text = await Text(join(directory, name));
		if (text !== null) return { name, text };
	}
	return null;
};

/**
 * One skill's record: the directory name is the skill's name, because that is the
 * address a reader types and an index cites.
 */
const ReadSkill = async (root: string, name: string): Promise<Skill> => {
	const directory = join(root, name);
	const body = (await Text(join(directory, BodyFile))) ?? "";
	const { data } = SplitFrontmatter(body);
	return {
		name,
		directory,
		description: data?.description?.trim() ?? "",
		references: await References(directory),
	};
};

/**
 * A stateless contract: every call goes to the filesystem, nothing is kept.
 *
 * `digest` is the one exception, and it is the opposite of a cache - it is a pure
 * function of the records it is handed, so it can be computed for a corpus that
 * is not on this machine at all.
 */
export const Create = (): SkillContract => ({
	async discover(root) {
		const entries = await Entries(root);
		const skills: Skill[] = [];
		for (const entry of entries) {
			if (!(await IsDirectory(entry, join(root, entry.name)))) continue;
			skills.push(await ReadSkill(root, entry.name));
		}
		return skills.sort((left, right) => Compare(left.name, right.name));
	},

	async validate(skill) {
		const findings: Finding[] = [];
		const report = (rule: string, detail: string): void => {
			findings.push({ skill: skill.name, rule, detail });
		};

		const body = await Text(join(skill.directory, BodyFile));
		if (body === null) {
			report("unparsable", `${skill.directory} carries no ${BodyFile}`);
		} else {
			const { data, error } = SplitFrontmatter(body);
			if (error !== null || data === null) {
				report("unparsable", `the frontmatter does not parse: ${error}`);
			} else {
				const description = data.description?.trim() ?? "";
				if (description === "") {
					report("unparsable", "the frontmatter states no description");
				}
				const name = data.name?.trim() ?? "";
				if (name !== "" && name.toLowerCase() !== skill.name.toLowerCase()) {
					report(
						"unparsable",
						`the frontmatter names ${JSON.stringify(name)} but the directory is ${JSON.stringify(skill.name)}`,
					);
				}
			}
		}

		const index = await Text(join(skill.directory, ReferenceDirectory, IndexFile));
		const references = await References(skill.directory);
		for (const reference of references) {
			if (index === null) {
				report(
					"unlisted-reference",
					`${ReferenceDirectory}/${reference} is not named in ${ReferenceDirectory}/${IndexFile}, because there is no ${ReferenceDirectory}/${IndexFile}`,
				);
			} else if (!Names(index, reference)) {
				report(
					"unlisted-reference",
					`${ReferenceDirectory}/${reference} is not named in ${ReferenceDirectory}/${IndexFile}`,
				);
			}
		}

		const corpus = await CorpusIndex(dirname(skill.directory));
		if (corpus === null) {
			report(
				"uncited",
				`the corpus root carries no ${CorpusIndexes.join(" or ")}, so nothing points at ${skill.name}`,
			);
		} else if (!Names(corpus.text, skill.name)) {
			report("uncited", `${corpus.name} does not name ${skill.name}/`);
		}

		return findings;
	},

	digest(skills) {
		const canonical = skills.map((skill) => [skill.name, skill.description, skill.references]);
		const bytes = createHash("sha256").update(JSON.stringify(canonical), "utf8").digest("hex");
		return `sha256:${bytes}`;
	},
});
