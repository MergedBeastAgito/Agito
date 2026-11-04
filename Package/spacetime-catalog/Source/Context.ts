/**
 * The context a reducer is given, and the whole of it.
 *
 * This is the seam that makes the rules testable without SpacetimeDB. The
 * consuming module's `ctx` has far more on it - an identity type, a timestamp
 * type, indexes, views, scheduling - and none of that belongs in a rule. What a
 * rule needs is five things: who is calling, what time it is, and a way to read,
 * insert, update and delete one row by key.
 *
 * Two decisions that look like omissions and are not:
 *
 * - `sender` is a `string`, not the module's `Identity`. An adapter renders
 *   `ctx.sender` once, at the boundary, and every rule compares that rendering.
 *   A rule that compared an identity to a caller-supplied argument would be
 *   comparing two different things.
 * - `now` is a `number` in whatever unit the module's clock uses, and no rule
 *   reads a wall clock. A reducer that reads `Date.now()` cannot be replayed,
 *   and a test cannot decide what time it is.
 *
 * `delete` is declared because the context has it. No rule in `Reducers.ts`
 * deletes a row; the fake in `../Test/Fake.ts` records a delete as a write and
 * refuses one on an append-only table, so a rule that started deleting history
 * would fail a test rather than pass review.
 */

/** Every value a column may hold. `null` is how an optional column is absent. */
export type Value = string | number | boolean | null;

/** One row, keyed by column name. The catalog in `./Tables.ts` describes it. */
export type Row = Readonly<Record<string, Value>>;

export interface Context {
	/** The caller. The recipient of a reveal, the holder of a lease, the author
	 * of a trace - never an identity supplied in an argument. */
	readonly sender: string;
	/** The module's time for this reducer call. Every timestamp a row carries
	 * comes from here. */
	readonly now: number;
	/** The row keyed `id` in `table`, or `null`. One lookup, no scan. */
	find(table: string, id: string): Row | null;
	/** Insert a row. A row already at its key is a defect, not an overwrite. */
	insert(table: string, row: Row): void;
	/** Replace the row at the key `row` carries. A missing row is a defect. */
	update(table: string, row: Row): void;
	/** Remove the row at `id`. */
	delete(table: string, id: string): void;
}
