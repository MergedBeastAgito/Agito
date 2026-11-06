---
name: ui-step-sequence
description: Use when a destructive UI flow must run as gated steps. One expression, one report.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, ui, automation, step-sequence, portal]
        related_skills: [browser-eval, bounded-sequences]
---

# UI Step Sequence

A destructive flow - remove this item: kebab, menu item, confirm dialog - fails quietly when it is
driven as a chain of clicks whose misses go unreported. The kebab of the wrong row opens a menu, the
menu closes before the click lands, the confirm button is found inside a hidden dialog, and the call
returns success over an item that is still there. One expression that walks the flow stage by stage
and returns the stage it reached turns that into a report that names itself.

The verb is `agito browse --expression`. What one expression may do is `Browser-Eval/`; what several
operations in order cost is `Bounded-Sequences/`.

---

## Locate by stable ID, never by display name

A display name is not an identity. Two rows can read the same, a row can be renamed between the read
and the click, and a truncated name matches a prefix of another. Take the identifier out of a link
the row already carries - a record id or a URN in a query parameter - and match on that. The row's
own anchor is the honest source: `a[href*="<id-param>"]`, then the parameter out of the decoded
`href`.

The same rule governs the opener. Find the row's overflow button by `aria-label` prefix and scope
the query to the row, so the first kebab in the document is not the one clicked.

---

## The stages, and what each one refuses

1. **Row** - the one whose id matches. Missing is `row_not_found`, and nothing is clicked.
2. **Opener** - the row's kebab, by `aria-label` prefix. Missing is `opener_not_found`.
3. **Menu** - queried at document level, filtered to a visible rect, first interactive child
   clicked. Missing is `menu_not_found`.
4. **Confirm** - a visible `button` whose trimmed `innerText` is the confirm label. Missing is
   `confirm_not_found`.
5. **Report** - `{"step": "done", "stillThere": <bool>}`, where `stillThere` re-runs the row lookup.

Every stage returns before the next one starts. A stage that throws or clicks a stale node is worse
than a stage that reports absence, because the next stage then acts on whatever the page did
instead.

**The menu is in a portal.** A dropdown renders outside the row that opened it, so a query scoped to
the row finds nothing and a query scoped to the row's parent finds the wrong menu. Query
`[role="menu"]` at document level and keep only the node with a non-zero rect - the closed menus
from earlier interactions are still in the DOM.

**A rect is the cheap visibility test and it has two blind spots.** `getBoundingClientRect()`
reports zero for a `display: contents` wrapper that renders its children and for a
`content-visibility: auto` subtree that is offscreen. Where a menu is built that way, test the
interactive child's own rect rather than the container's.

---

## Prototype one stage per expression

Write the row lookup, run it, read the id it resolved. Then add the opener. Then the menu. A wrong
selector in the middle of a finished chain does not fail where it is wrong: it fails at the confirm
stage, or it clicks the menu of the row above, and the report says `done`. One stage per run costs
four round trips and localises every mistake to the stage that made it.

---

## Waits belong to the stage that needs them

A single-page app re-renders between stages, so a click followed immediately by a query reads the
pre-click DOM. Measured defaults: about 900 ms after the kebab, 1.5 s after the menu item, 3 s after
the confirm. They are waits for a re-render, not for a network answer, so they do not grow with the
payload.

**The call's budget has to cover the waits.** `--timeout` is the whole expression's budget, so a
sequence that sleeps 5.4 s in total inside a 5 s budget is killed mid-chain and reported as
`timeout` - indistinguishable from a page that never answered, and it leaves the flow half-executed.
Pass a budget with room: 60000 ms for the sequence below.

---

## Template

Write the sequence to a file and pass its contents, rather than quoting thirty lines of JavaScript
into a shell argument.

```sh
Target/release/agito browse \
	--url https://example.test/items \
	--attach ws://127.0.0.1:<debug-port>/devtools/browser/<uuid> \
	--bind <target-id> \
	--expression "$(cat <path>/Step.js)" \
	--timeout 60000
```

```js
(async () => {
	const visible = (Element) => {
		const Rect = Element.getBoundingClientRect();
		return Rect.width > 0 && Rect.height > 0;
	};
	const idOf = (Row) => {
		const Anchor = Row.querySelector('a[href*="<id-param>"]');
		return Anchor ? decodeURIComponent(Anchor.href).match(/<param>=([^&]+)/)?.[1] : "";
	};
	const findRow = () =>
		[...document.querySelectorAll("<row-selector>")].find((Row) => idOf(Row) === "<ID>");
	const row = findRow();
	if (!row) return JSON.stringify({ step: "row_not_found" });
	const opener = row.querySelector('button[aria-label^="More actions"]');
	if (!opener) return JSON.stringify({ step: "opener_not_found" });
	opener.click();
	await new Promise((Resolve) => setTimeout(Resolve, 900));
	const menu = [...document.querySelectorAll('[role="menu"]')].find(visible);
	if (!menu) return JSON.stringify({ step: "menu_not_found" });
	menu.querySelector("button, li, [role=menuitem], a").click();
	await new Promise((Resolve) => setTimeout(Resolve, 1500));
	const confirm = [...document.querySelectorAll("button")].find(
		(Button) => visible(Button) && Button.innerText.trim() === "<confirm-label>",
	);
	if (!confirm) return JSON.stringify({ step: "confirm_not_found" });
	confirm.click();
	await new Promise((Resolve) => setTimeout(Resolve, 3000));
	return JSON.stringify({ step: "done", stillThere: !!findRow() });
})();
```

`menu.querySelector("button, li, [role=menuitem], a")` takes the first interactive child, which is
right only for a one-item menu. Where the menu carries several actions, select the item by its
trimmed text and report `menu_not_found` when no item matches - "the first child" is how a remove
flow ends up pressing Duplicate.

The report is a JSON **string**: `JSON.stringify` keeps it one value the expression can return, and
the string is what a caller parses.

---

## `done` is not the outcome

The report proves the chain was walked, not that the item left. `stillThere` re-reads the DOM the
sequence itself just changed, which is the same surface the click acted on - two defects can cancel
into `done`. Close the loop against something the flow did not produce: a fresh expression that
counts the rows, the list endpoint's own answer, or the row's absence after a reload. See
`Bounded-Sequences/` for why the second read is the one that counts.

---

## Claims and how to test them

| Claim                                          | Test                                                                                                                                                                  |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A missing row is reported, not clicked through | run with `<ID>` set to an id the list does not hold; the value is `{"step":"row_not_found"}` and the page is unchanged                                                |
| The opener belongs to the matched row          | two rows whose kebabs open different menus; the clicked menu names the matched row's action                                                                           |
| A portal menu is found from document level     | the menu opens while the row is still mounted; the expression reads `[role="menu"]` off `document`                                                                    |
| A closed menu is not clicked                   | with an earlier menu still in the DOM, `visible` rejects it and the sequence reaches the confirm stage                                                                |
| The confirm button is the visible one          | two dialogs mounted, one hidden; the clicked button is the one with a non-zero rect                                                                                   |
| The budget covers the waits                    | `--timeout 60000` against 5.4 s of sleeps; `--timeout 3000` reports `timeout` mid-chain                                                                               |
| An async sequence returns its report as data   | `--expression '(async () => ({step: "done"}))()'` prints that object; an empty object means the evaluate path did not await the promise, and the stages must be split |
| `done` is not proof the item left              | a fresh expression counts the rows after the call; `stillThere` false with the row still present is the failure                                                       |
