# TOML probe: is the file, or the parser, at fault

The question: a configuration file failed to parse, and the report's candidate causes were an em
dash (U+2014) inside a comment, a multibyte value, or a UTF-8 BOM. Three hypotheses, one run, plus a
control that is genuinely broken so the error shape on real breakage is visible beside the false
ones. The same run checks what `tracing` does with a `warn!` when no subscriber is installed.

```toml
[package]
name = "toml-probe"
version = "0.1.0"
edition = "2024"

[dependencies]
toml = "1.1.6"
tracing = "0.1"
```

`src/main.rs` parses four embedded strings and prints one marker per case:

```rust
fn main() {
	// Exact file from the report: em dash (U+2014) inside a comment.
	let em_dash =
		"[compression]\nterminal_threshold = 16384   # bytes; was default 1024 \u{2014} raised\n";
	match em_dash.parse::<toml::Table>() {
		Ok(t) => {
			let v = t
				.get("compression")
				.and_then(|c| c.as_table())
				.and_then(|c| c.get("terminal_threshold"));
			println!("EM_DASH_COMMENT: PARSE_OK, terminal_threshold={v:?}");
		}
		Err(e) => println!("EM_DASH_COMMENT: PARSE_ERR: {e}"),
	}

	// A multibyte character as a VALUE too, which is the stronger case.
	let multibyte_value = "[compression]\nname = \"héllo - wörld\"\n";
	match multibyte_value.parse::<toml::Table>() {
		Ok(_) => println!("MULTIBYTE_VALUE: PARSE_OK"),
		Err(e) => println!("MULTIBYTE_VALUE: PARSE_ERR: {e}"),
	}

	// Genuinely broken TOML: what the error actually looks like when a
	// file IS broken. This is the control.
	let broken = "[compression\nenabled = false\n";
	match broken.parse::<toml::Table>() {
		Ok(_) => println!("BROKEN: PARSE_OK (unexpected)"),
		Err(e) => println!("BROKEN: PARSE_ERR: {e}"),
	}

	// UTF-8 BOM at the start of the file, which some editors write.
	let bom = "\u{FEFF}[compression]\nterminal_threshold = 16384\n";
	match bom.parse::<toml::Table>() {
		Ok(_) => println!("BOM: PARSE_OK"),
		Err(e) => println!("BOM: PARSE_ERR: {e}"),
	}

	// tracing with no subscriber: a warn! is a silent no-op.
	println!("tracing has_been_set={}", tracing::dispatcher::has_been_set());
	tracing::warn!("this warn is dropped when no subscriber is installed");
	eprintln!("stderr fallback line IS visible");
}
```

The em dash is written as the escape `\u{2014}` so the byte survives an editor, a diff and a copy:
the string the probe parses is the report's line exactly, and the source stays ASCII.

Run: `cargo run -q` inside the probe directory. The markers answered:

| Case              | Marker                               | What it settles                                                      |
| ----------------- | ------------------------------------ | -------------------------------------------------------------------- |
| `EM_DASH_COMMENT` | `PARSE_OK` with the value read back  | the em dash in a comment was not the cause                           |
| `MULTIBYTE_VALUE` | `PARSE_OK`                           | a multibyte value was not the cause either                           |
| `BROKEN`          | `PARSE_ERR` with a line and column   | this is what a real parse error looks like, and it names a position  |
| `BOM`             | `PARSE_OK`                           | `toml` 1.1.6 accepts a leading UTF-8 BOM, so it is not the cause     |
| `tracing`         | `has_been_set=false`, no warn output | `warn!` is dropped without a subscriber; `eprintln!` is the fallback |

Measured output, on `toml 1.1.6+spec-1.1.0`:

```
EM_DASH_COMMENT: PARSE_OK, terminal_threshold=Some(Integer(16384))
MULTIBYTE_VALUE: PARSE_OK
BROKEN: PARSE_ERR: TOML parse error at line 1, column 13
  |
1 | [compression
  |             ^
unclosed table, expected `]`

BOM: PARSE_OK
tracing has_been_set=false
stderr fallback line IS visible
```

**The control is what makes the other three readable.** A `PARSE_OK` on its own only says this input
was accepted. The `BROKEN` case prints the error's shape - a line, a column, a caret under the
offending byte and a sentence naming the construct - and that is what distinguishes "rejected for a
reason" from "rejected by a different parser than the one being blamed". Without it, an error on the
em dash case would have been read as confirmation of the report's first hypothesis.

**The BOM is a candidate the report did not name, and this version clears it.** The answer was
expected the other way, from a memory of the crate rejecting a leading `\u{FEFF}`; `PARSE_OK` is
what the run printed. That is the whole argument for the probe: a remembered behaviour is a claim
about some version, and the marker is a statement about the one the tree resolves. A version that
rejects the BOM prints `PARSE_ERR` with the same control's shape, so the case costs one line either
way.

**Reading the value back, not just the verdict.** The em dash case prints `terminal_threshold={v:?}`
from the parsed table, so `PARSE_OK` is accompanied by the number the file declared. A parser that
accepts a file and drops the key would pass a bare `PARSE_OK`.

**What the `tracing` half answers.** `has_been_set=false` is the state a binary is in when nothing
installed a subscriber, and in that state a `warn!` reaches nobody: the line is dropped, not
buffered and not printed to stderr. A tool that reports a problem through `warn!` alone is silent in
exactly the configuration where the problem matters. `eprintln!` is visible with or without a
subscriber, which is why the fallback line is there.
