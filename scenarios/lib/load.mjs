/**
 * The two things every side of the differential run does to a scenario file:
 * read it, and compare what it observed against what the file expects.
 *
 * The comparison is deliberately blunt. A scenario is either exactly what the
 * file says - the same return value, the same ordered effect list, the same
 * number of reports - or it is a mismatch with a sentence saying which of the
 * three disagreed. There is no tolerance, because the point of the file is to be
 * the single source both implementations answer to.
 */
import { readFileSync } from "node:fs";

/** Read one scenario file. */
export const load = (path) => JSON.parse(readFileSync(path, "utf8"));

/** Merge one level of `patch` into `base`, section by section. */
export const merge = (base, patch) => {
	if (patch === null || patch === undefined) return base;
	const merged = { ...base };
	for (const [section, values] of Object.entries(patch)) {
		merged[section] =
			values !== null && typeof values === "object" && !Array.isArray(values)
				? { ...(base[section] ?? {}), ...values }
				: values;
	}
	return merged;
};

/** Every case in a file, with the file's fixture folded in. */
export const casesOf = (file) =>
	file.cases.map((entry) => ({
		...entry,
		fixture: merge(file.fixture, entry.fixture),
	}));

const same = (left, right) => JSON.stringify(left) === JSON.stringify(right);

/**
 * What one observation got wrong, as sentences. Empty means the case agreed.
 *
 * `expect.result` is compared field by field rather than as one blob, so a
 * mismatch says which field drifted; `message` is a substring, because the two
 * implementations word the same fact differently by design (the Rust port has no
 * `40ms` in its deadline sentence, the TypeScript one does) and the fact - the
 * code, and that the sentence mentions the deadline - is what the port promises.
 */
export const compare = (observed, expected) => {
	const wrong = [];

	const want = expected.result;
	const got = observed.result;
	if ((want?.kind ?? null) !== (got?.kind ?? null)) {
		wrong.push(`result kind: wanted ${want?.kind}, got ${got?.kind}`);
	} else if (want?.kind === "error") {
		if (want.code !== got.code) wrong.push(`result code: wanted ${want.code}, got ${got.code}`);
		if (want.message !== undefined && !String(got.message ?? "").includes(want.message))
			wrong.push(
				`result message: wanted to contain ${JSON.stringify(want.message)}, got ${JSON.stringify(got.message)}`,
			);
		// `observed` is the one token that must survive somewhere in the failure.
		// The two ports split a sentence and an observation differently - the
		// TypeScript one writes its own sentence and keeps the reason in `detail`,
		// the Rust one has a code and no room for a fourth kind of failure, so its
		// launcher's sentence is the message - and what the port promises is that
		// the observation is not lost, not which field it sits in.
		if (
			want.observed !== undefined &&
			!`${got.message ?? ""} ${got.detail ?? ""}`.includes(want.observed)
		)
			wrong.push(
				`result observation: wanted ${JSON.stringify(want.observed)} in message or detail, got ${JSON.stringify(`${got.message ?? ""} / ${got.detail ?? ""}`)}`,
			);
	} else if (want?.kind === "target") {
		for (const field of ["ownership", "url", "ws"])
			if (want[field] !== got[field])
				wrong.push(
					`result ${field}: wanted ${JSON.stringify(want[field])}, got ${JSON.stringify(got[field])}`,
				);
	} else if (want?.kind === "value") {
		if (!same(want.value, got.value))
			wrong.push(
				`result value: wanted ${JSON.stringify(want.value)}, got ${JSON.stringify(got.value)}`,
			);
	} else if (want?.kind === "answer") {
		for (const field of ["text", "model", "mode"])
			if (want[field] !== undefined && want[field] !== got[field])
				wrong.push(
					`result ${field}: wanted ${JSON.stringify(want[field])}, got ${JSON.stringify(got[field])}`,
				);
		for (const field of ["trace", "sources"])
			if (want[field] !== undefined && !same(want[field], got[field]))
				wrong.push(
					`result ${field}: wanted ${JSON.stringify(want[field])}, got ${JSON.stringify(got[field])}`,
				);
	} else if (want?.kind === "findings") {
		if (!same(want.rules, got.rules))
			wrong.push(
				`result rules: wanted ${JSON.stringify(want.rules)}, got ${JSON.stringify(got.rules)}`,
			);
	} else if (want?.kind === "names") {
		if (!same(want.names, got.names))
			wrong.push(
				`result names: wanted ${JSON.stringify(want.names)}, got ${JSON.stringify(got.names)}`,
			);
	} else if (want?.kind === "digest") {
		if (want.equals !== undefined && want.equals !== got.value)
			wrong.push(
				`result digest: wanted ${JSON.stringify(want.equals)}, got ${JSON.stringify(got.value)}`,
			);
		if (want.matches !== undefined && !new RegExp(want.matches).test(String(got.value)))
			wrong.push(
				`result digest: wanted to match ${want.matches}, got ${JSON.stringify(got.value)}`,
			);
	}

	if (expected.effects !== undefined && !same(expected.effects, observed.effects))
		wrong.push(
			`effects: wanted ${JSON.stringify(expected.effects)}, got ${JSON.stringify(observed.effects)}`,
		);

	if (expected.reports !== undefined && expected.reports !== observed.reports)
		wrong.push(`reports: wanted ${expected.reports}, got ${observed.reports}`);

	return wrong;
};

/** The runner for one capability, from a table of them. */
export const runner = (table, capability) => {
	const found = table[capability];
	if (found === undefined) throw new Error(`no runner for ${capability}`);
	return found;
};
