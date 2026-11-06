# Reference

Files in this directory, and what each one carries.

- `Index.md` - this list.
- `Regex-Probe.md` - four log-filter patterns compiled in a scratch crate: the `ALL_OK` marker, why
  a per-pattern line would report partial success, and what compile-validity does not answer.
- `Toml-Probe.md` - the four-case TOML probe (em dash in a comment, multibyte value, a genuinely
  broken file as the control, a UTF-8 BOM) plus what `tracing` does with a `warn!` and no
  subscriber.
