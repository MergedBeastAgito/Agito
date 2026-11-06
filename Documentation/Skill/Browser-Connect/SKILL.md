---
name: browser-connect
description: Use when reaching a running browser over CDP from Rust. Attach, bind a tab, launch.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, rust, chromiumoxide, attach, target]
---

# Browser Connect

Reach a browser this process did not start, and bind the tab a person is looking at. The Rust
substrate is `agito-browser-cdp`; the executable is `agito browse`.

---

## The three operations

They differ in who owns the process and which page is touched, so they are three statements, not one
boolean.

**Launch** starts a browser this session owns and may close. `OpenOptions::Launch(call)`.

**Attach** connects to a browser somebody else owns and opens a new page in it.
`OpenOptions::Attach(endpoint, call)`.

**Bind** connects to that browser and drives a page it already has.
`OpenOptions::Existing(endpoint, page, call)`.

Disposal reads the ownership: a launched browser is closed, an attached one is only let go of. A
bind that closed the page would take away the tab the person was reading, which is the whole reason
ownership is recorded.

---

## Find the endpoint

Two endpoints exist and they are not interchangeable. `/json/version` carries the browser-level one
that attach needs; `/json/list` carries per-tab ones and is the only honest tab inventory.

```sh
curl -s http://127.0.0.1:<debug-port>/json/version | python3 -c 'import sys,json;print(json.load(sys.stdin)["webSocketDebuggerUrl"])'
curl -s http://127.0.0.1:<debug-port>/json/list | python3 -c 'import sys,json;[print(t["id"],"|",t["type"],"|",t["url"][:80]) for t in json.load(sys.stdin)]'
```

The `id` from `/json/list` is the target id a bind takes. Filter `type == "page"`: a listing also
carries extension pages, service workers and iframes, and a bind by URL prefix matches a non-page.

---

## Bind an existing tab

```sh
Target/debug/agito browse \
  --url https://example.test/ \
  --attach ws://127.0.0.1:<debug-port>/devtools/browser/<uuid> \
  --bind <target-id> \
  --expression '({title: document.title, url: location.href})' \
  --timeout 40000
```

`--bind` without `--attach` is refused, because a browser this tool launched has no page to bind.
The refusal is deliberate: silently opening a tab instead is the failure this flag exists to
prevent.

**Why a bind needs a fetch first.** A browser announces no `targetCreated` event for the tabs that
were already open when a client connected, so the handler's own map is empty and `Browser::pages()`
returns nothing. `Chrome::Bind` calls `Browser::fetch_targets()` before matching, which registers
those targets and starts an `AttachToTarget` round trip per target.

**Why the match is retried.** A page cannot be handed out until its session arrives, so a first
match can legitimately miss. The match is retried every 25 ms up to a 2 s grace
(`Chrome::PageGrace`); past it, the failure names every page the browser does hold, which is how a
race is told apart from a wrong id:

```
navigation-failed: the browser holds no page matching the one asked for
no target id or URL starts with AAAA...; it holds [1111... https://... | 2222... chrome-extension://...]
```

Matching is target id first, URL prefix second. The id is exact and the prefix is not, so a caller
that knows only the URL gets the first page that starts with it.

**What a bind costs.** `fetch_targets` attaches a CDP session to _every_ target in that browser. On
a browser that also hosts an extension holding its own debugger, a second client on a tab can detach
the first. Bind deliberately, on the tab you mean, and do not enumerate a browser you do not intend
to drive.

---

## Failure codes

The port has three codes and the substrate maps every `chromiumoxide` error onto one of them, so a
caller never has to guess which library failed.

| Code                  | Means                                                  | Typical cause                                                        |
| --------------------- | ------------------------------------------------------ | -------------------------------------------------------------------- |
| `browser-unavailable` | there is no browser to talk to                         | socket closed, connect refused, process exited, no answer            |
| `navigation-failed`   | the transport is alive and the page is what went wrong | JavaScript exception, page not found, protocol error naming the page |
| `timeout`             | a deadline passed                                      | the caller's own budget, or silence                                  |

A closed transport wins over a page defect: once the socket is gone, the next failure is classified
`browser-unavailable` rather than as the page misbehaving.

---

## Never attach to a tab a debugger already holds

Two debugger clients cannot share one tab; the second attach detaches the first. If a tab is being
driven by an extension or another session, drive a different tab or a browser you launched. This is
a correctness rule, not a courtesy.

---

## Reference

`Reference/Index.md` - the file list for this skill.

---

## Claim → test

| Claim                                       | Test                                                                                                                           |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| A bind drives the page that is already open | `agito browse --attach <ws> --bind <id> --expression 'location.href'` returns that tab's URL, and `/json/list` gains no target |
| An attach without a bind opens a page       | the same command without `--bind` makes `/json/list` show one more `page` target                                               |
| A bind without an attach is refused         | `agito browse --bind <id>` exits non-zero naming `--attach`                                                                    |
| A wrong target id is reported, not guessed  | `--bind deadbeef` exits non-zero and lists the pages the browser holds                                                         |
| Ownership is honoured on release            | after a bind, the bound tab is still in `/json/list`; after a launch, the browser is gone                                      |
| The browser-level endpoint is the one used  | attaching with a per-tab `/json/list` websocket fails as `browser-unavailable`                                                 |
