/**
 * @type {import('prettier').Config}
 *
 * The formatting half of what this repository used to have. The TypeScript
 * capabilities are gone, but the prose, the manifests and the workflows are not,
 * and a repository that cannot format its own documentation is one where a
 * reviewer reads whitespace instead of a change.
 *
 * Two decisions that look cosmetic but are not:
 *
 * - `useTabs: true` keeps indentation one byte per level, so a deep tree costs
 *   less in diffs and a screen reader can be told a tab width.
 * - `endOfLine: "lf"` is load-bearing: a shebang script checked out with CRLF
 *   reads `env: bash\r`, the interpreter is not found, and the script silently
 *   does not run.
 *
 * Rust is not formatted here. `rustfmt.toml` owns `.rs` and `.prettierignore`
 * excludes it, because two formatters claiming one file is how a file becomes
 * unformatted by both.
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
