---
name: paginated-scrape
description: Use when a list must be read to its end. Paginate, key every row by stable ID, export.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, rust, scraping, pagination, dataset, deduplication]
        related_skills: [browser-eval, bounded-sequences, batch-mutation-verify]
---

# Paginated Scrape

A scrape that stops when the page shows no more cards reports a truncated list as a complete one,
and a scrape keyed on display names merges two rows or drops one. This skill is the read that
produces a deduped dataset with a stable ID per row - the target list a later batch mutates
(`Batch-Mutation-Verify/`).

`agito browse` is the mechanism: one expression per page, evaluated against the bound tab, returning
its value as JSON. The crawl is a loop the caller owns, one `--timeout` per page, so the page that
hangs is named rather than the run.

---

## 1. Detect the card

Find the container that holds ONE item: it carries both the item's stable-ID link and its action
buttons. Do not trust one selector - walk up from the name link until the element also contains the
action link carrying the stable ID. That walk is what survives a class rename, and it is why the
container is defined by what it contains rather than by how it is spelled.

---

## 2. Extract the fields and a stable ID

Per card: the display name, the classification fields (location, category), the profile or record
URL, and a **stable ID** taken from an action-link parameter - `recipient=<urn>` is one shape of it.

The ID is both the dedupe key and the join key across pages: it is what lets page 4's row be
recognised as page 2's row, and what a later mutation matches on.

Watch for **nested and mutual links sharing the parent's ID**: a card whose avatar, title and action
button all point at the same record yields three elements per row. Dedupe by ID and keep the primary
row - the first link in document order - so the row's name is the name and not an image's alt text.

The expression returns data, never prose: an immediately-invoked function returning an object, so
one round trip answers the whole card. `undefined` arrives as `null`, and `null` is
indistinguishable from "the page does not have it" unless the expression says which.

---

## 3. Paginate to the terminal page

- **Paged.** Find the **Next** button and read `aria-disabled` or `disabled` for the last page. Wait
  for the URL's page parameter to change before scraping: a click that returns `true` has not yet
  re-rendered the list, and the read then captures the previous page under the new URL.
- **Infinite scroll or "Load more".** Click **Load more** (which adds a batch) and/or scroll the
  **correct container** - often a nested element with its own overflow, not `window`. Stop when
  neither the click nor the scroll grows the card count across several attempts.

A repeated key is the honest end of the walk, and the attempt cap is a second, independent stop: a
count is a guess about a page that may add rows while it is being read, while a repeated ID is
evidence of having come back around (`Bounded-Sequences/`).

---

## 4. Classify, then export

Group by a classification field, then write **JSON** (machine), **CSV** and **Markdown** (human)
into the workspace the user designated, never the system temp directory. The export is the review
gate before any destructive step, so it names every row it holds and the field each row was
classified on.

---

## Pitfalls

- **Match classification against a dictionary keyed on the field's actual value**, applied only to
  the correct line. "Last comma segment of a line" breaks the moment a headline or another card
  field contains a comma, and it breaks silently by misclassifying rather than by failing.
- **The card DOM may virtualise.** Rows that scroll off are dropped from the document, so a single
  render pass sees a window and not a list. Dedupe across pages and accumulate into a map keyed by
  stable ID.
- **A selector that matches is not a selector that is unique.** Count the matches; one card per
  match is a property to verify, not to assume.
- **A page parameter that has not changed is a page not yet loaded.** Wait on the value, not on the
  click, and give the wait its own budget so a page that never advances ends the crawl rather than
  the run.

---

## Claims and how to test them

| Claim                                | Probe                                                                | Pass condition                                           |
| ------------------------------------ | -------------------------------------------------------------------- | -------------------------------------------------------- |
| The container holds exactly one item | count matches of the walked-up selector                              | the count equals the number of visible cards             |
| The stable ID is stable              | scrape page 1 twice, minutes apart                                   | the same IDs in the same order, or an explained diff     |
| Nested links collapse to one row     | count rows against IDs                                               | one row per ID, named from the primary link              |
| A page is only read after it changed | log the page parameter beside each capture                           | no two captures share a parameter                        |
| The walk reaches the end             | compare the final count with the app's own total, where it shows one | equal, or the difference is explained by a filtered view |
| A virtualised list loses nothing     | accumulate across pages, then count the map                          | no ID appears twice and none is missing from the export  |
| The export is reviewable             | open the Markdown file                                               | every row shows name, ID, classification and record URL  |
