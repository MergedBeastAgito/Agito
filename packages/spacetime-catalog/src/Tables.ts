/**
 * The catalog: the six tables of the agentic model, as data.
 *
 * A SpacetimeDB module compiles its tables into the published module, so a row
 * in a table-definition table cannot create one, and no reducer can execute DDL.
 * What can be shared instead is the description: name, columns, types, keys, and
 * whether a table is append-only. A module's `Schema.ts` renders its
 * `table(...)` declarations from this, and a reader diffs two versions of this
 * file to see what a generation changed.
 *
 * `appendOnly` is a rule rather than a storage detail. The fake context in
 * `../Test/Fake.ts` refuses an update or a delete on a table marked append-only
 * here, so "the previous generation is untouched" is enforced by the same data a
 * module would render - not by a comment.
 *
 * Times are `number` in whatever unit the module's `ctx.timestamp` uses. The
 * rules only add, compare and store them, so the unit belongs to the module; a
 * `ttl` passed to `claim` or `renew` must be in that same unit.
 */

/**
 * Every type a column may have. The list is data because a test asserts that
 * every column uses one of these, which is what makes a typo in a column a
 * failure rather than a silently unrenderable declaration.
 */
export const ColumnTypes = [
	"string",
	"number",
	"boolean",
	"json",
	"optional-string",
	"optional-number",
] as const;

export type ColumnType = (typeof ColumnTypes)[number];

export type Column = {
	/** The column name, which is also the key of the row object. */
	readonly name: string;
	/** The value type a module declaration renders. */
	readonly type: ColumnType;
	/** `primary` for the key `find` and `update` address a row by, `index` for
	 * the keys a module would index, `null` for a plain column. */
	readonly key: "primary" | "index" | null;
	/** What the column means, and the rule it carries when it carries one. */
	readonly note: string;
};

export type Table = {
	readonly name: string;
	/** True when a row may be inserted but never updated or deleted. */
	readonly appendOnly: boolean;
	readonly columns: readonly Column[];
};

export type TableName =
	"capability" | "generation" | "lease" | "probe" | "belief" | "trace";

/**
 * The six tables, in the order a reader meets them: what may be bound, what was
 * bound, who holds what, what was disclosed, what was inferred, what happened.
 */
export const Tables: Readonly<Record<TableName, Table>> = {
	/**
	 * The capabilities a manifest may name. Append-only: a new version is a new
	 * row, so a generation's manifest keeps naming what it named.
	 *
	 * Rows are declared by the module's `init` from its own package manifests.
	 * There is deliberately no reducer that declares a capability: a declaration
	 * is a deployment, and a reducer that could add one would let a caller widen
	 * the set its own manifest is checked against.
	 */
	capability: {
		name: "capability",
		appendOnly: true,
		columns: [
			{
				name: "id",
				type: "string",
				key: "primary",
				note: "`<name>@<version>` - the identifier a manifest names",
			},
			{
				name: "name",
				type: "string",
				key: "index",
				note: "the stable capability name, without a version",
			},
			{
				name: "version",
				type: "string",
				key: null,
				note: "the version this declaration pins",
			},
			{
				name: "state",
				type: "string",
				key: null,
				note: "stateless | holds-transport | holds-identity | holds-workspace",
			},
			{
				name: "declaredAt",
				type: "number",
				key: null,
				note: "ctx.now at declaration, which is the module's init",
			},
			{
				name: "declaredBy",
				type: "string",
				key: null,
				note: "who declared it, so a declaration has provenance",
			},
		],
	},

	/**
	 * Every generation that has been bound. Append-only, and that is what makes
	 * the binding history auditable: a generation is superseded by the existence
	 * of its successor, never by a column someone rewrote.
	 *
	 * Ids are sequence numbers, `1`, `2`, ... - dense by construction, because
	 * `bind` computes a successor's id as its parent's sequence plus one. That
	 * density is load-bearing: it is what lets "is this the latest bound
	 * generation?" be answered with one key lookup in a context that cannot scan.
	 */
	generation: {
		name: "generation",
		appendOnly: true,
		columns: [
			{
				name: "id",
				type: "string",
				key: "primary",
				note: "the sequence number as a decimal string: `1`, `2`, ...",
			},
			{
				name: "sequence",
				type: "number",
				key: null,
				note: "the same number as a number, so a successor's id needs no parsing",
			},
			{
				name: "parent",
				type: "optional-string",
				key: null,
				note: "the generation this one supersedes; null for the first",
			},
			{
				name: "manifestDigest",
				type: "string",
				key: null,
				note: "the composition root's name for the manifest it bound",
			},
			{
				name: "capabilities",
				type: "json",
				key: null,
				note: "the capability ids the manifest names, sorted, as a JSON array",
			},
			{
				name: "boundAt",
				type: "number",
				key: null,
				note: "ctx.now at bind",
			},
			{
				name: "boundBy",
				type: "string",
				key: null,
				note: "ctx.sender at bind",
			},
		],
	},

	/**
	 * One row per resource, current state. Not append-only: a claim and a renew
	 * rewrite this row, which is the whole point of a lease.
	 *
	 * The row is never deleted. Deleting it would erase the fence, and a fence
	 * that restarts at one is a fence a late worker can satisfy. Epoch history
	 * belongs in `trace`; the row holds only the current epoch.
	 */
	lease: {
		name: "lease",
		appendOnly: false,
		columns: [
			{
				name: "id",
				type: "string",
				key: "primary",
				note: "the resource id, e.g. `browser:default` or `task:<id>`",
			},
			{
				name: "generation",
				type: "string",
				key: "index",
				note: "the generation the lease was granted under; a lease never adopts a later one",
			},
			{
				name: "holder",
				type: "string",
				key: "index",
				note: "the sender that holds it - the only sender a fenced write is accepted from",
			},
			{
				name: "task",
				type: "string",
				key: null,
				note: "the attempt the lease was granted for",
			},
			{
				name: "fence",
				type: "number",
				key: null,
				note: "incremented by every claim; a write carrying an older fence is refused",
			},
			{
				name: "state",
				type: "string",
				key: null,
				note: "held | released",
			},
			{
				name: "acquiredAt",
				type: "number",
				key: null,
				note: "ctx.now at the claim that opened this epoch",
			},
			{
				name: "renewedAt",
				type: "optional-number",
				key: null,
				note: "null until the first renew of this epoch",
			},
			{
				name: "expiresAt",
				type: "number",
				key: null,
				note: "acquiredAt + ttl; every fenced write compares it to ctx.now, because an expired lease cannot be renewed back to life",
			},
		],
	},

	/**
	 * What a specific audience learned about a specific property version, and
	 * when. Append-only, because a reveal that can be rewritten is not evidence
	 * of what was revealed.
	 *
	 * There is no column for the private value, and no reducer here is ever given
	 * it. A row states the fact that was disclosed, to whom, at which version -
	 * so the secret is not merely filtered from a reader, it never enters the
	 * database.
	 */
	probe: {
		name: "probe",
		appendOnly: true,
		columns: [
			{
				name: "id",
				type: "string",
				key: "primary",
				note: "`<property>:<version>:<audience>` - the reveal is its own receipt, so a second probe at the same version finds this row",
			},
			{
				name: "property",
				type: "string",
				key: "index",
				note: "the property that was probed",
			},
			{
				name: "version",
				type: "number",
				key: null,
				note: "the property version the observation is pinned to; seeing the current truth is a separate entitlement",
			},
			{
				name: "audience",
				type: "string",
				key: "index",
				note: "ctx.sender of the probe - a caller-supplied audience would be a caller-supplied identity",
			},
			{
				name: "observed",
				type: "string",
				key: null,
				note: "what was disclosed, never the private value it was computed from",
			},
			{
				name: "at",
				type: "number",
				key: null,
				note: "ctx.now at the reveal",
			},
			{
				name: "generation",
				type: "string",
				key: "index",
				note: "the generation the reveal was made under",
			},
		],
	},

	/**
	 * What an audience inferred from the observations it holds. Not append-only,
	 * because a belief is disposable: deleting one loses nothing that cannot be
	 * recomputed from the probe rows it names.
	 *
	 * A belief grants no access. It is the actor's own reading, which is why it
	 * may carry an interpretation at all while `probe` may not carry the truth.
	 */
	belief: {
		name: "belief",
		appendOnly: false,
		columns: [
			{
				name: "id",
				type: "string",
				key: "primary",
				note: "`<audience>:<property>:<version>` - one current belief per audience",
			},
			{
				name: "audience",
				type: "string",
				key: "index",
				note: "the actor the belief belongs to, and the only actor it is about",
			},
			{
				name: "property",
				type: "string",
				key: null,
				note: "the property being inferred",
			},
			{
				name: "version",
				type: "number",
				key: null,
				note: "the property version the belief was formed at",
			},
			{
				name: "interpretation",
				type: "string",
				key: null,
				note: "the audience's own reading of its observations; never the private truth",
			},
			{
				name: "derivedFrom",
				type: "string",
				key: null,
				note: "the probe row it was derived from, so it can be rebuilt",
			},
			{
				name: "at",
				type: "number",
				key: null,
				note: "ctx.now when the belief was derived",
			},
		],
	},

	/**
	 * Append-only log of what happened, keyed by content so a replay appends
	 * nothing. Evidence, not authority: a trace records a decision after it was
	 * taken, and is the one place a refused write can still leave a mark.
	 */
	trace: {
		name: "trace",
		appendOnly: true,
		columns: [
			{
				name: "id",
				type: "string",
				key: "primary",
				note: "`<at>:<kind>:<subject>` - a replay at the same instant is the same fact",
			},
			{
				name: "at",
				type: "number",
				key: null,
				note: "ctx.now when it was recorded",
			},
			{
				name: "kind",
				type: "string",
				key: "index",
				note: "what happened; a closed set the module owns",
			},
			{
				name: "subject",
				type: "string",
				key: "index",
				note: "what it happened to - a resource, a generation, a property",
			},
			{
				name: "payload",
				type: "string",
				key: null,
				note: "opaque JSON, so a new event shape needs no migration; never a private value",
			},
			{
				name: "generation",
				type: "string",
				key: "index",
				note: "the generation the event happened under",
			},
			{
				name: "sender",
				type: "string",
				key: null,
				note: "ctx.sender that recorded it",
			},
		],
	},
};

/** The table names, in the order a reader meets them. */
export const TableNames: readonly TableName[] = [
	"capability",
	"generation",
	"lease",
	"probe",
	"belief",
	"trace",
];

/**
 * The column a row is keyed by, which is what `find`, `insert`, `update` and
 * `delete` address a row with. It reads the description rather than assuming
 * `id`, so a table whose key is not called `id` cannot silently be keyed wrongly
 * by a context that guesses.
 *
 * Throws on a table the catalog does not describe, and on a table without
 * exactly one primary key: both are defects in the catalog, not conditions a
 * reducer can recover from.
 */
export const PrimaryKey = (table: string): string => {
	const described = (Tables as Record<string, Table | undefined>)[table];
	if (described === undefined) throw new Error(`no such table: ${table}`);
	const primary = described.columns.filter(
		(column) => column.key === "primary",
	);
	const [key] = primary;
	if (key === undefined || primary.length !== 1)
		throw new Error(`${table} does not declare exactly one primary key`);
	return key.name;
};
