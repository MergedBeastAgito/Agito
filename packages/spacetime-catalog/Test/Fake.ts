/**
 * A context in memory, with three properties a real one has and a test needs.
 *
 * 1. **It records every write.** A refusal that still wrote is the failure the
 *    fence and coherence rules exist to prevent, so the assertion is usually
 *    "and the fake recorded nothing".
 * 2. **It refuses to rewrite history.** A table the catalog marks append-only
 *    cannot be updated or deleted here, so "the previous generation is untouched"
 *    is enforced rather than asserted.
 * 3. **It checks every row against the catalog.** A write must name described
 *    columns, all of them, with the described types. A private value therefore
 *    cannot enter the database through a column the catalog never declared - and
 *    a reducer that mistyped a column fails a test instead of passing review.
 *
 * The clock is `now`, which the test moves. Nothing reads a wall clock, so a
 * sequence of reducer calls at a chosen time is a transcript the test controls
 * completely.
 */
import {
	PrimaryKey,
	Tables,
	type Column,
	type Context,
	type Row,
	type TableName,
	type Value,
} from "../src/index.ts";

export type Write = {
	readonly kind: "insert" | "update" | "delete";
	readonly table: string;
	readonly id: string;
	readonly row: Row | null;
};

const Described = (table: string) => {
	const described = (Tables as Record<string, unknown>)[table];
	if (described === undefined) throw new Error(`no such table: ${table}`);
	return described as (typeof Tables)[TableName];
};

const Conforms = (table: string, column: Column, value: Value): void => {
	const wrong = () => {
		throw new Error(`${table}.${column.name} is ${column.type}, got ${JSON.stringify(value)}`);
	};
	switch (column.type) {
		case "string":
		case "json":
			if (typeof value !== "string") wrong();
			return;
		case "number":
			if (typeof value !== "number") wrong();
			return;
		case "boolean":
			if (typeof value !== "boolean") wrong();
			return;
		case "optional-string":
			if (value !== null && typeof value !== "string") wrong();
			return;
		case "optional-number":
			if (value !== null && typeof value !== "number") wrong();
			return;
	}
};

const Check = (table: string, row: Row): void => {
	const described = Described(table);
	const columns = new Map(described.columns.map((column) => [column.name, column]));
	for (const [name, value] of Object.entries(row)) {
		const column = columns.get(name);
		if (column === undefined) throw new Error(`${table}.${name} is not a described column`);
		Conforms(table, column, value);
	}
	for (const column of described.columns) {
		if (!(column.name in row))
			throw new Error(`${table}.${column.name} is missing from the row`);
	}
};

export class Memory implements Context {
	sender: string;
	now: number;
	readonly writes: Write[] = [];
	readonly #tables = new Map<string, Map<string, Row>>();

	constructor(sender = "agent:a", now = 0) {
		this.sender = sender;
		this.now = now;
	}

	/** Become another caller on the same database, as a second agent would be. */
	as(sender: string): this {
		this.sender = sender;
		return this;
	}

	find(table: string, id: string): Row | null {
		return this.#rows(table).get(id) ?? null;
	}

	insert(table: string, row: Row): void {
		Check(table, row);
		const id = this.#id(table, row);
		const rows = this.#rows(table);
		if (rows.has(id)) throw new Error(`insert of an existing row: ${table}/${id}`);
		rows.set(id, row);
		this.writes.push({ kind: "insert", table, id, row });
	}

	update(table: string, row: Row): void {
		if (Described(table).appendOnly)
			throw new Error(`update of an append-only table: ${table}`);
		Check(table, row);
		const id = this.#id(table, row);
		const rows = this.#rows(table);
		if (!rows.has(id)) throw new Error(`update of a missing row: ${table}/${id}`);
		rows.set(id, row);
		this.writes.push({ kind: "update", table, id, row });
	}

	delete(table: string, id: string): void {
		if (Described(table).appendOnly)
			throw new Error(`delete from an append-only table: ${table}`);
		const rows = this.#rows(table);
		if (!rows.delete(id)) throw new Error(`delete of a missing row: ${table}/${id}`);
		this.writes.push({ kind: "delete", table, id, row: null });
	}

	/** Every row of one table, in the order it was written. */
	rows(table: string): Row[] {
		return [...this.#rows(table).values()];
	}

	/** The id a row is keyed by, read from the catalog rather than assumed. */
	#id(table: string, row: Row): string {
		const key = PrimaryKey(table);
		const value = row[key];
		if (typeof value !== "string") throw new Error(`${table}.${key} is not a string key`);
		return value;
	}

	#rows(table: string): Map<string, Row> {
		const existing = this.#tables.get(table);
		if (existing !== undefined) return existing;
		const created = new Map<string, Row>();
		this.#tables.set(table, created);
		return created;
	}
}
