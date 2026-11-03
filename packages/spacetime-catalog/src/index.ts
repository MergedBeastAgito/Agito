/**
 * @agentic/spacetime-catalog - The agentic database model as rules and data.
 *
 * The tables are data (`./Tables.ts`), the context a reducer needs is one
 * interface (`./Context.ts`), and the rules are five pure functions
 * (`./Reducers.ts`). Nothing here imports `spacetimedb/server`: a module renders
 * its declarations from the catalog and satisfies the context, and the rules are
 * the same rules on both sides of that seam.
 */

export type {
	Column,
	ColumnType,
	Table,
	TableName,
} from "./Tables.ts";
export { ColumnTypes, PrimaryKey, TableNames, Tables } from "./Tables.ts";
export type { Context, Row, Value } from "./Context.ts";
export type {
	Bind,
	Claim,
	Probe,
	ProbeResult,
	Reason,
	Refusal,
	Renew,
	Result,
	Trace,
} from "./Reducers.ts";
export { bind, claim, Latest, probe, recordTrace, renew } from "./Reducers.ts";
