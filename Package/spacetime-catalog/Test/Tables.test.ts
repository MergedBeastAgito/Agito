/**
 * The catalog read as data.
 *
 * These tests are about what a module would render and what a reader would diff:
 * the six tables, their keys, their types, and which of them may be rewritten.
 * The rule that the probe table cannot hold a private value is a column check
 * here rather than a convention in a comment, because a column is what a reader
 * can select.
 *
 *   node --test Test/*.test.ts
 */
import assert from "node:assert/strict";
import { test } from "node:test";

import { ColumnTypes, PrimaryKey, TableNames, Tables } from "../Source/index.ts";

test("the catalog describes exactly the six tables the model needs", () => {
	assert.deepEqual(TableNames, ["capability", "generation", "lease", "probe", "belief", "trace"]);
	assert.deepEqual(Object.keys(Tables), [...TableNames]);
	for (const name of TableNames) assert.equal(Tables[name].name, name);
});

test("every table declares one primary key, and it is the column rows are addressed by", () => {
	for (const name of TableNames) {
		const primary = Tables[name].columns.filter((column) => column.key === "primary");
		assert.equal(primary.length, 1, `${name} declares ${primary.length} keys`);
		assert.equal(PrimaryKey(name), primary[0]?.name);
	}
	assert.deepEqual(
		TableNames.map((name) => PrimaryKey(name)),
		["id", "id", "id", "id", "id", "id"],
	);
});

test("every column uses a type the catalog declares", () => {
	for (const name of TableNames) {
		for (const column of Tables[name].columns) {
			assert.ok(
				(ColumnTypes as readonly string[]).includes(column.type),
				`${name}.${column.name} is ${column.type}`,
			);
			assert.ok(column.note.length > 0, `${name}.${column.name} has no note`);
		}
	}
});

test("only the two current-state tables are mutable; history is append-only", () => {
	assert.deepEqual(
		TableNames.filter((name) => !Tables[name].appendOnly),
		["lease", "belief"],
	);
});

test("the probe table has no column that could hold the private value", () => {
	const columns = Tables.probe.columns.map((column) => column.name);
	for (const forbidden of ["value", "truth", "secret", "private", "raw"])
		assert.ok(
			!columns.includes(forbidden),
			`probe.${forbidden} would make the secret selectable`,
		);
	assert.deepEqual(columns, [
		"id",
		"property",
		"version",
		"audience",
		"observed",
		"at",
		"generation",
	]);
});

test("no table in the catalog describes a column named for a secret", () => {
	for (const name of TableNames) {
		for (const column of Tables[name].columns)
			for (const forbidden of ["secret", "token", "password", "cookie"])
				assert.ok(
					!column.name.toLowerCase().includes(forbidden),
					`${name}.${column.name} names a credential`,
				);
	}
});

test("an undescribed table is a defect, not an empty answer", () => {
	assert.throws(() => PrimaryKey("nowhere"), /no such table/);
});
