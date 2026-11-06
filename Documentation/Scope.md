# Scope, and the parity with TimeLine

Agito is not TimeLine's toolkit. It drives a browser and the chat products in it from Rust: one
binary browses, asks a question and reads the answer back, measures what a page rendered, and
validates a corpus of skill documents, and the same work is a workspace a product embeds through one
crate. TimeLine is a product - an archive, a site, a decision corpus and the tooling that maintains
them.

They share a vocabulary - capabilities, ports, leases, evidence - and one interface: a research
finding from Agito becomes a card in the DELO project. Nothing travels the other way, and nothing
here is required for TimeLine to build.

## What Agito is

| Capability            | What it does                                                             | State class   |
| --------------------- | ------------------------------------------------------------------------ | ------------- |
| `browser-cdp`         | attach to or launch a browser, evaluate in a page, dispose               | HoldTransport |
| `chat-driver`         | ask a chat product, verify mode and model, read the answer and its trace | HoldTransport |
| `design-verification` | measure a built page and produce evidence, with no design baked in       | stateless     |
| `skill-contract`      | discover, validate and digest a corpus of skill documents                | stateless     |
| `runtime`             | admission, leases with a fence, per-run budgets, cleanup                 | HoldWorkspace |
| `spacetime-catalog`   | the database rules: capabilities, generations, leases, probes, trace     | stateless     |

## What TimeLine is

| Surface          | What it does                                                                              |
| ---------------- | ----------------------------------------------------------------------------------------- |
| the archive      | posts, authors, observations, media, membership - written only through `observe`          |
| the site         | a prerendered Astro surface that reads the archive and never scrapes                      |
| the RFC corpus   | 76 decision records, their frontmatter contract and the issue board that publishes them   |
| design verifiers | three probes that measure the built workspace, showcase and spread against a written spec |
| the tool layer   | 125 TypeScript modules: CLI, transport, store, model, command                             |
| the gates        | `pnpm run check`: RFC frontmatter, skills, compatibility, state, tokens, position, tests  |

## Parity: what one has that the other does not

| Concern                           | Agito                                                              | TimeLine                                     |
| --------------------------------- | ------------------------------------------------------------------ | -------------------------------------------- |
| A browser session                 | yes - attach or launch, with ownership recorded                    | yes, but only to verify its own pages        |
| Asking a chat product             | yes - mode and model verified, trace captured                      | no                                           |
| Research corpus                   | yes - threads captured as records with frontmatter and a catalogue | no                                           |
| A published site                  | no                                                                 | yes                                          |
| An archive of records             | no                                                                 | yes                                          |
| A decision corpus with a contract | no                                                                 | yes - 76 records, gated                      |
| Issue-board synchronisation       | no                                                                 | yes - `rfc_board.py`, read-only by default   |
| Leases and admission              | yes - the runtime's whole purpose                                  | no                                           |
| Design measurement                | yes - the engine, with no design in it                             | yes - the engine plus the site's own numbers |
| Skill validation                  | yes - generic corpus rules                                         | yes - its own 13 skills and their contract   |
| Persistence                       | SpacetimeDB, as the scheduler and the record                       | files plus SpacetimeDB for the live view     |

## The interface between them

A research finding from Agito becomes a card in the DELO board, and the card is written in
TimeLine's terms: a file, a claim, a command that settles it. That is the whole interface. Agito
does not import TimeLine's code, does not read its database, and is not a dependency of its build.

## The boundary

- Agito carries no site, no archive and no RFC board.
- TimeLine carries no copy of the runtime, the chat driver or the research corpus.
- A capability that both need - measurement, skill validation - lives in Agito as the engine and in
  TimeLine as the site's own expectations, which is why the two versions of `design-verification`
  are not duplicates.
