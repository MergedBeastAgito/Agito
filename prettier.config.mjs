/** @type {import('prettier').Config} */

/**
 * The same formatting rules as the sibling TimeLine repository: tabs for
 * indentation, 100 columns, double quotes, semicolons, trailing commas.
 *
 * Two decisions that look cosmetic but are not:
 *
 * - `useTabs: true` keeps indentation one byte per level, so a deep tree costs
 *   less in diffs and screen readers can be told a tab width.
 * - `endOfLine: "lf"` is load-bearing here: a shebang script checked out with
 *   CRLF reads `env: bash\r`, the interpreter is not found, and the script
 *   silently does not run.
 *
 * Two entries from that file are deliberately absent, and both are absences of
 * need rather than disagreements:
 *
 * - No `prettier-plugin-astro` and no `*.astro` override. There is no site in
 *   this repository, so an Astro parser would be a dependency that formats
 *   nothing.
 * - No `@ianvs/prettier-plugin-sort-imports`. It reorders imports, which is a
 *   real formatting behaviour, but it is also the one rule here that rewrites a
 *   line's meaning for a reviewer rather than its whitespace. The capability
 *   packages are the specification the Rust port is compared against, and their
 *   import lists are read in the order they are written. Add the plugin and its
 *   devDependency together if that changes.
 *
 * The config is dependency-free: `prettier` itself is the only package this file
 * needs, which is why it is the only devDependency the root declares.
 */
export default {
	printWidth: 100,
	useTabs: true,
	tabWidth: 4,
	semi: true,
	singleQuote: false,
	trailingComma: "all",
	bracketSameLine: true,
	bracketSpacing: true,
	arrowParens: "always",
	endOfLine: "lf",
	proseWrap: "always",
	quoteProps: "preserve",

	htmlWhitespaceSensitivity: "css",
	jsxSingleQuote: false,
	vueIndentScriptAndStyle: true,
	embeddedLanguageFormatting: "auto",

	overrides: [
		{
			// Prettier's json parser accepts comments; keep output standard.
			files: ["*.json", "*.jsonc"],
			options: { parser: "json", trailingComma: "none" },
		},
		{
			// YAML has no tab indentation.
			files: ["*.{yaml,yml}"],
			options: { useTabs: false, tabWidth: 2 },
		},
	],
};
