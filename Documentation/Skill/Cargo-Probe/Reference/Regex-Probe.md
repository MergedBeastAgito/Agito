# Regex probe: do four log filters compile

The question: four log-line filter patterns were about to be embedded in a tool, and an invalid
pattern embedded there is a runtime panic on whichever path first builds it. The probe asks the
`regex` crate about all four before anything is embedded.

```toml
[package]
name = "regex-check"
version = "0.1.0"
edition = "2024"

[dependencies]
regex = "1"
```

`src/main.rs` compiles each pattern with `Regex::new(...).expect(...)` and prints `ALL_OK` only
after the last one:

```rust
use regex::Regex;

fn main() {
	let pats = [
		r"(?i)\b(?:ERROR|FAIL|FAILED|FATAL|CRITICAL)\b\s*(?:[:\[(]|\])",
		r"\b(?:ERROR|FAIL|FAILED|FATAL|CRITICAL)\b",
		r"(?i)\b(?:WARN|WARNING)\b\s*(?:[:\[(]|\])",
		r"\b(?:WARN|WARNING)\b",
	];
	for p in pats {
		Regex::new(p).expect(p);
	}
	println!("ALL_OK");
}
```

Run: `cargo run -q` inside the probe directory.

**Why `ALL_OK` and not a per-pattern line.** `Regex::new` panics on the first invalid pattern, and
`expect(p)` puts the offending pattern in the panic message. So the absence of `ALL_OK` localises
the failure to the pattern named in the panic, and the presence of it is a single unambiguous fact
about all four. A per-pattern `OK` line would report three successes and one panic, which reads as
partial success for a set that has to be all-or-nothing.

The case-insensitivity is worth noting as a probe result in itself: `(?i)` compiles and applies to
the whole pattern, so the two lowercase-free patterns match `error` and `Error` alike. The two
without `(?i)` are the case-sensitive halves, and the pair is what makes the filter's behaviour
explicit rather than assumed.

**What it does not answer.** A pattern that compiles can still be wrong: it can match the wrong
lines, or fail to match the ones the log actually carries. Compile-validity is one question and
matching behaviour is a second one, taken over real log lines; this probe answers only the first.
