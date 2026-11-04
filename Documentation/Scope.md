# Scope, and the parity with TimeLine

Agentic is not TimeLine's toolkit. It is a local agent and swarm: it gathers links, browses,
researches, and organises that work in SpacetimeDB. TimeLine is a product - an archive, a site, a
decision corpus and the tooling that maintains them.

They share a vocabulary (capabilities, ports, leases, evidence) and one direction of travel:
**research done here can be fed into the DELO project as findings.** Nothing travels the other way,
and nothing here is required for TimeLine to build.

## What Agentic is

| Capability            | What it does                                                             | State class     |
| --------------------- | ------------------------------------------------------------------------ | --------------- |
| `browser-cdp`         | attach to or launch a browser, evaluate in a page, dispose               | holds-transport |
| `chat-driver`         | ask a chat product, verify mode and model, read the answer and its trace | holds-transport |
| `design-verification` | measure a built page and produce evidence, with no design baked in       | stateless       |
| `skill-contract`      | discover, validate and digest a corpus of skill documents                | stateless       |
| `runtime`             | admission, leases with a fence, per-run budgets, cleanup                 | holds-workspace |
| `spacetime-catalog`   | the database rules: capabilities, generations, leases, probes, trace     | stateless       |

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

| Concern                           | Agentic                                                            | TimeLine                                     |
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

## The one direction of travel

A research finding from Agentic becomes a card in the DELO board, and the card is written in
TimeLine's terms: a file, a claim, a command that settles it. That is the whole interface. Agentic
does not import TimeLine's code, does not read its database, and is not a dependency of its build.

## What this rules out

- Agentic does not grow a copy of the site, the archive or the RFC board.
- TimeLine does not grow a copy of the runtime, the chat driver or the research corpus.
- A capability that both need - measurement, skill validation - lives in Agentic as the engine and
  in TimeLine as the site's own expectations, which is why the two versions of `design-verification`
  are not duplicates.
