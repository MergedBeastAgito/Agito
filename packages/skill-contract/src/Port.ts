/**
 * The port `@agentic/skill-contract` promises.
 *
 * A corpus is a directory of skill directories. A skill is a directory carrying
 * a `SKILL.md` and, when it has worked instances, a `Reference/` tree. A skill's
 * `Reference/` carries an `Index.md` and the corpus root carries an index, and
 * each index names its siblings: the root's names the skills, a skill's names its
 * files. The root's index is `Index.md`, or `AGENTS.md` when the corpus grew
 * inside a repository that already gives that name to the file its tools
 * discover; when both exist `Index.md` is read.
 *
 * Those two indexes are the whole structure this port assumes, because they are
 * the two places a reader is sent - a corpus nobody indexes is entered by
 * guessing, and a reference an index omits is a dead end.
 *
 * The port states more than data shapes, so the semantics two adapters would
 * otherwise disagree about are fixed here rather than left to an implementation:
 *
 * - `discover` is deterministic. Two runs over the same tree return the same
 *   order, sorted by skill name rather than by whatever the filesystem answered,
 *   because directory order is not stable across platforms or file systems.
 * - `validate` reports, it does not throw. A malformed skill is a finding, so one
 *   bad directory cannot abort a sweep over a corpus of good ones.
 * - `digest` is a pure function of the records it is handed. It reads no file, so
 *   a modification time cannot enter it, and `directory` is excluded from it, so
 *   the same corpus digests the same in two checkouts.
 * - Stateless means stateless. Nothing is cached between calls, and `validate`
 *   re-reads the skill's directory instead of trusting the record it is given: a
 *   record is a claim about a directory, and a cache of that claim is how a sweep
 *   reports yesterday's corpus.
 */

export type Skill = {
	name: string;
	directory: string;
	description: string;
	references: string[];
};

export type Finding = {
	skill: string;
	rule: string;
	detail: string;
};

export interface SkillContract {
	/** Every skill directory under the corpus root. */
	discover(root: string): Promise<Skill[]>;
	/** Every rule a skill breaks. An empty array is a valid corpus. */
	validate(skill: Skill): Promise<Finding[]>;
	/** A stable digest of the corpus: names, order and reference lists. */
	digest(skills: Skill[]): string;
}

/**
 * The three rules `validate` reports, and the only three it may report.
 *
 * They mirror `provides.errors` in `module.json`, which is what a harness reads;
 * the contract test compares the two so the manifest cannot drift from the code.
 *
 * - `unparsable` - the frontmatter does not describe the skill that directory is:
 *   no `SKILL.md`, no frontmatter block, a block that is not a mapping, no
 *   `description`, or a `name` that disagrees with the directory. The directory is
 *   the skill's address, so a frontmatter name that contradicts it is a rename
 *   that stopped halfway.
 * - `unlisted-reference` - a file under `Reference/` that `Reference/Index.md`
 *   does not name, which is a reader's dead end.
 * - `uncited` - the corpus root's index - `Index.md`, or `AGENTS.md` - does not
 *   name this skill, which is a skill no reader is pointed at. The corpus root is
 *   the skill directory's parent, so `validate` can apply the rule to a record
 *   without being told where the corpus begins.
 *
 * Two rules the source implementation enforces are deliberately not here: whether
 * the body carries a claim-to-test table, and whether a second, reader-facing
 * index names the skill. Both are policies of one repository rather than
 * properties of a corpus, and a port that hard-codes them would refuse every
 * corpus that does not share that repository's house style.
 */
export const Rules = ["unparsable", "unlisted-reference", "uncited"] as const;

export type Rule = (typeof Rules)[number];
