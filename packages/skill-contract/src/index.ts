/**
 * @agentic/skill-contract - Discover, validate and package a skill corpus.
 *
 * The port this package promises is declared in ../module.json. Nothing here may
 * import another package's adapter; the composition root supplies one.
 */

export type { Finding, Rule, Skill, SkillContract } from "./Port.ts";
export { Create } from "./SkillContract.ts";
export { Rules } from "./Port.ts";
