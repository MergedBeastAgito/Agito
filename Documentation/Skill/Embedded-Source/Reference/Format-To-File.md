# One expression, inline and then in a file

`ReadMode` is the smallest complete instance of the move: one `querySelectorAll`, one parameter, one
return value. It is the expression to read first when adding another, because every part of the
contract is visible in it at once.

## The file

`Crate/Chat/Driver/Source/Script/Expression/ReadMode.ts`:

```ts
declare const __PARAM_Mode__: string;

(() => {
	const Label = (Element) =>
		(Element.getAttribute("aria-label") || Element.innerText || "").trim().toLowerCase();
	const Pressed = [...document.querySelectorAll(__PARAM_Mode__)].filter(
		(Toggle) => Toggle.getAttribute("aria-pressed") === "true",
	);
	return Pressed.length === 1 ? Label(Pressed[0]) : null;
})();
```

Three things to notice, and each is a rule in `SKILL.md`:

- the declaration is line 1, followed by one blank line, because the erasure removes the declaration
  with its line and the blank line after it;
- the selector is the marker `__PARAM_Mode__` in an argument position, which is where an identifier
  is valid - not a `{{Mode}}`, and not a property key;
- the quotes inside are `"`, which is what the page receives. Nothing in this file exists to satisfy
  Rust, because Rust is not the language it is written in.

## The call site

```rust
pub fn ReadMode(Selectors:&Selectors) -> String { script!("Expression/ReadMode"; Mode = &Selectors.Mode) }
```

`Mode` is the name without the marker and the trailing underscores; the value is the caller's
selector. The argument list is compared with the file's declarations in a `const` context, so
`Mode = ...` missing, added twice, or misspelt is a `cargo build` error pointing at this line.

## What the build writes

Two artefacts per script under `Target/debug/build/<package>/<hash>/out/Script/Expression/`:

```
ReadMode.js      # the page's copy
ReadMode.params  # one `Name:kind` line per declaration
```

`ReadMode.params` holds exactly:

```
Mode:text
```

`ReadMode.js` holds the source with the declaration line and the blank line after it removed, and
every other byte unchanged - the same tabs, the same quotes, the same trailing semicolon. Verified
by:

```sh
diff <(tail -n +3 Crate/Chat/Driver/Source/Script/Expression/ReadMode.ts) \
	Target/debug/build/agito-chat-driver/*/out/Script/Expression/ReadMode.js
```

Silence is the pass. That is the property a transpiler cannot give you: `oxc_transformer` appends a
semicolon after the spliced marker and re-indents to one tab, `esbuild` drops the comment block and
rewrites `undefined` to `void 0`, `tsc` prepends `"use strict";` - each of them a change in what a
page receives, for no gain in correctness.

## Where the marker becomes a value

The `.js` artefact still carries `__PARAM_Mode__`; the copy in the binary does not. `script!`
expands to `include_str!` of that file plus `FillDeclared` with the parameter list, evaluated in a
`const` context, so the value is substituted at compile time and a `string` parameter is escaped
into a JavaScript string literal on the way. That is why a missing argument is `E0080` at the call
site and not a `ReferenceError` in a page, and why an unfilled marker is only reachable from a call
that passes no parameters at all.

## The inline shape this replaced

A `format!` version of the same expression is recognisable by what it costs: every brace doubled,
every quote escaped, the selector interpolated as a Rust value.

```rust
format!("(() => {{ const Pressed = [...document.querySelectorAll({})].filter((T) => T.getAttribute(\"aria-pressed\") === \"true\"); return Pressed.length === 1 ? (Pressed[0].innerText || \"\").trim().toLowerCase() : null; }})()", Literal(&Selectors.Mode))
```

Nothing in that line is wrong, and that is the problem: no editor syntax-highlights the JavaScript,
no linter sees it, no compiler parses it, and a single missing backslash produces a page error with
no build error. The one thing the string form buys is locality, and the file form buys it back with
a path that is one line long.
