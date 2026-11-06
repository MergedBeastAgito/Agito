# Style

One convention for names, and the boundary of it. `CONTRIBUTING.md` carries the short table; this is
the whole statement, including the cases where something outside this tree fixes a spelling.

---

## The rule

**A name states one concept.** PascalCase for every name - folder, file, module, type, function,
field, parameter, constant, JSON key, prose heading. A name that would carry two concepts is two
names, and therefore two folders.

**A verb names a leaf.** An operation that acts on an object puts the object in the folder and the
verb in the file: `Fn/Module/Verify.rs` holds `Verify`, `Fn/Lane/Select.rs` holds `Select`,
`Fn/Skill/Page/Verify.rs` holds `Verify`. A verb is present tense and as short as the action is.

**A folder names a role, singular.** `Source/Fn`, `Source/Struct`, `Source/Script`, `Crate/Chat`,
`Documentation/Skill`. A plural is a collection, not a role: `Entries` becomes `Entry`, `Names`
becomes `Name`, `Expressions` becomes `Expression`.

**A compound that names one thing stays one word, and both parts are capitalised.** `RunTime`,
`StandIn`, `FutureBox`, `Workflow`, `SDK`. `Runtime` is the defect: it spells one word where the
language has two, which is why the crate, the module and the type are all `RunTime` now.

**An acronym is fully cased, in names and prose.** `SDK`, `URL`, `URI`, `ID`, `API`, `JSON`, `HTTP`,
`WASM`, `CDP`, `HTML`, `CSS`, `SQL`. `CdpClient` is `CDPClient`; `AgitoSdk` is `SDK`.

**A serialized key is PascalCase, singular and an action where it names one.** `Provide` not
`Provides`, `Require` not `Requires`, `Error` not `errors`, `Verb` not `verbs`, `Process` not
`processes`. A key is a name like any other, and the reader of a `module.json` is a person first.

**A crate's directory leaf is its library name.** Rust cannot nest a crate root, so the group folder
carries the object and the leaf carries the name: `Crate/Chat/Driver` is library `Driver`,
`Crate/Design/Verification` is library `Verification`, `Crate/Browser/CDP` is library `CDP`.

**A parameter is PascalCase, one word, singular where it holds one value.** `Path`, `Root`, `Call`,
`Module`, `Selectors` (a collection), `Adapter` (repeatable). A parameter that needs a sentence is
two parameters.

**A link to a submodule names its own repository, never a path inside the parent.** `Use/NFSU2` is a
repository, so the parent's URL with `tree/Current/NFSU2` appended to it is a 404 - GitHub has no
folder there to open - while `https://github.com/MergedBeastAgito/NFSU2-Use` is the repository
itself. A file inside the submodule is that repository's `blob` URL, not the parent's: the branch
does not rescue the parent's path, because `tree/Current/…`, `tree/main/…` and `blob/Current/…` are
one defect, and the map from a submodule path to its repository is the `.gitmodules` of the tree
that holds it.

---

## The boundary

A name is not ours to change when something outside the tree already fixes it. The exceptions are
listed rather than discovered:

- **Cargo package ids stay lowercase kebab** - `agito-chat-driver`, `agito-runtime`. Cargo requires
  it, and `-p agito-chat-driver` is the spelling a caller types. The directory and the library carry
  the PascalCase form; the package id does not.
- **A Rust crate root cannot be nested** - the leaf is the library, the group folder is the path.
- **A dependency's API keeps its spelling** - the Chrome DevTools Protocol domain
  `Runtime.evaluate`, `chromiumoxide`'s types, `oxc`'s AST nodes, `serde`'s attributes. Renaming one
  would break a call, not a convention.
- **Dotted port ids and scoped package ids stay lowercase** - `agito.chat.ask`,
  `@agito/chat-driver`. They are wire identifiers a manifest carries, not names a reader scans.
- **Database table and column names stay lowercase** - `Crate/Spacetime/Catalog` states a schema a
  SpacetimeDB module publishes; a column renamed for style is a migration.
- **The template corpus keys stay lowercase** - `version`, `fields`, `lanes`, `templates`, `order`
  in `Templates/Observation.json` and `Questions.json`. That file is data an ingestion writes.
- **The skill corpus keeps `<Root>/<Name>/SKILL.md`** with frontmatter `name` and `description`.
  That layout is the format the validator implements, not a choice this tree makes.
- **The ecosystem's root files keep their names** - `Cargo.toml`, `module.json`, `LICENSE`,
  `.editorconfig`, `SKILL.md`.

---

## Applying it

1. **Inventory before editing.** List every path and identifier the rule touches, with the rule that
   applies. A pass that starts from `grep` and ends at `grep` leaves the directories no pattern
   named.
2. **Move with `git mv`, then follow what breaks.** A folder rename breaks imports, `Cargo.toml`
   member globs and path dependencies, `module.json` artefact entries, `include!` paths, `script!`
   arguments, and every path quoted in `Documentation/`. All of them are in the same change.
3. **Build between phases.** `cargo build --release -p agito` after each group of moves; the tree
   does not resolve its own imports half-renamed.
4. **Grep the residue, not the plan.** `grep -rIn` for each old token over the tree afterwards, and
   assign every hit a decision: renamed, or deliberately kept with its boundary named.

---

## Claim to test

| Claim                                             | Probe                                                            | Pass condition                                     |
| ------------------------------------------------- | ---------------------------------------------------------------- | -------------------------------------------------- |
| No old vocabulary survives                        | `grep -rIn 'Agentic\|TimeLine-DELO\|ChatDriver\|BlockOn'`        | no hits outside this file                          |
| No lowercase role folder remains                  | `find Crate Documentation -type d`                               | every name PascalCase except the listed boundaries |
| Every crate builds under its new name             | `cargo build --release -p agito`                                 | `Finished`                                         |
| The manifest schema is the one the reader expects | `./Target/release/agito manifests` and `agito verify --module …` | ten rows, `Ready: true`, PascalCase keys           |
| The skill corpus still validates                  | `./Target/release/agito skills Documentation/Skill`              | `0 finding(s)`                                     |
| A consumer still builds against the facade        | `cargo build --release` in `Use/NFSU2/App/driver`                | `Finished`                                         |
| The tree carries no machine path                  | `git ls-files -z \| xargs -0 grep -In '/Volumes/\|/Users/'`      | no hits                                            |
