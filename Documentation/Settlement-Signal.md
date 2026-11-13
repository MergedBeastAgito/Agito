# Settlement by page signal, not by polling

A chat answer is finished when the page stops changing it. Today `agito chat` decides that by
reading the thread repeatedly and comparing - `StableReads` is 3, so a settled answer is read four
times in all - and it reaches the newest answer by pressing the page's own "scroll to end" control.
Both choices act on the person's tab: the reads cost a timer that must be budgeted for (`--timeout`,
raised to 420 s for a research answer), and the scroll control moves the viewport under whoever is
looking at it. A person who binds a tab and then scrolls it themselves is fighting the driver.

This note replaces both. The page observes itself and says when it is done; Agito listens and reads
once.

---

## The flow

1. **Bind.** `agito chat --attach <endpoint> --bind <url-prefix>` resolves one target. Unchanged.

2. **Arm.** After the question is typed and read back from the composer, Agito installs one
   page-side agent in that target. Arming is idempotent: a second call replaces the agent rather
   than stacking a second one, keyed on a `window.__Agito` generation counter.

3. **Observe.** The agent installs three observers over the answer container:
    - a `MutationObserver` (subtree, `characterData`, `childList`) - token growth;
    - a `ResizeObserver` - layout growth a mutation record can miss when a virtualised thread
      re-renders;
    - an `IntersectionObserver` - whether the answer is on screen.

    The `IntersectionObserver` is the only thing in the driver that reads scroll state, and it reads
    it; it never writes it. Its two jobs: tell the caller the person has the answer in view, and
    tell the caller the answer is off screen so the answer is read through the DOM - which needs no
    visibility - instead of being scrolled into view. `offscreen: true` in the result is the honest
    statement that the person's viewport was left alone.

4. **Quiet.** No mutation and no resize for `--quiet <ms>` (default 700) means settled. The quiet
   timer lives in the page, so it fires at the moment the page actually stops rather than at the
   next tick of a poll interval.

5. **Signal.** The agent calls a CDP binding registered with `Runtime.addBinding` before arming.
   Agito receives `Runtime.bindingCalled` on the socket it already holds. Payload:

    ```json
    {
    	"event": "busy|quiet|offscreen|error",
    	"turn": "<mark>",
    	"chars": 0,
    	"offscreen": false,
    	"at": 0,
    	"generation": 1,
    	"mutations": 0
    }
    ```

    `generation` says which installation of the agent sent it, so a stale one from before a re-arm
    can be told from the one listening now; `mutations` is how many changes it had counted. Neither
    decides anything - they are what makes a run diagnosable when the page and the driver disagree.

    Nothing is polled: while the page works the socket is idle, so there is no interval to tune and
    no budget to exhaust.

6. **Read once, and confirm.** On `quiet`, Agito performs one read with the existing expression and
   the existing selectors. The three-read rule survives as a _check_ on that read rather than as a
   waiting loop of its own: the text must be non-empty, its mark must not already have been in the
   thread before the question, the page must not say it is still working, and the same text must
   still be there at the next quiet window.

    The confirmation is not decoration. Measured on the reference product, a ten-item answer paused
    for longer than a 700 ms window three items in, with no stop control on screen, and a driver
    that trusted the first quiet window returned three items as a settled answer. Each confirmation
    is one more quiet signal - which costs nothing while the page is writing, because the mutations
    reset the window - so the rule only spends time at an ending.

    What remains is a limit of the page rather than of the rule. A browser throttles a background
    tab, so its rendering can stand still for several seconds while the answer is still arriving;
    measured with the same build, a ten-item answer came back with six. A tab a person is looking at
    is not throttled, and the answer that closes the gap properly is the page's own completion
    marker - `Selectors.Done` - which stays empty by default because a pattern that matches a
    duration label makes every turn look finished from its first token.

7. **Guards.** The page can be wrong, so three of them:
    - `--stall <ms>` (default 90 000) - no signal at all for that long is a stall, reported as
      `stalled` with the page's last state. This is a stall detector, not a wait budget.
    - `--timeout` stays as an absolute ceiling and reports `settled: false` on expiry. It never
      returns partial text as though it were settled.
    - If `Runtime.addBinding` or the observers are unavailable, the driver degrades to the
      stable-reads loop and says so in the result (`"mode": "polled"`). A missing signal is never a
      failure.

8. **Re-arm on navigation.** A bound target that navigates loses its binding. The driver re-arms
   once; a second loss is a named blocker.

---

## The invariants this buys

- **The person keeps their viewport.** No `scrollIntoView`, no press of the page's scroll-to-end
  control, no `Page.bringToFront`, no `Target.activateTarget`, no focus emulation on a tab a person
  is using.
- **Any agent, any harness.** The signal is a page-side contract with a stable binding name, so a
  caller that is not Agito can honour the same one, and a caller that is a person sees nothing move.
- **Any site.** Container discovery is generic: the largest text-bearing element that grew since
  submit, narrowed by the site's `Selectors` overrides when they match. A site that offers nothing
  still settles, because quiet is quiet.
- **No interval to tune.** The cost of a slow answer is the answer's own duration, not a budget that
  has to be guessed in advance.

---

## Where it lives

| Piece                       | Home                                                                                              |
| --------------------------- | ------------------------------------------------------------------------------------------------- |
| Binding + event stream      | `Crate/Browser/CDP` - `Session` exposes `Runtime.bindingCalled`                                   |
| Page-side agent (JS source) | `Crate/Chat/Driver/Source/Script/Agent/Arm.ts`, baked in by `Fn/Page/Agent.rs`                    |
| Payload type and parse      | `Crate/Chat/Driver/Source/Struct/Signal.rs`                                                       |
| Arm, wait, read once        | `Crate/Chat/Driver/Source/Fn/Settle.rs`                                                           |
| CLI                         | `--quiet <ms>`, `--stall <ms>`, `--poll` (force the old loop)                                     |
| Result                      | gains `"mode": "signal"` or `"polled"`, `"offscreen": bool`, `"stalled": bool`, `"settled": bool` |

The agent is a TypeScript file rather than a Rust string for the reason every expression here is:
the file a compiler parses is the file the page runs. `Fn/Page/Agent.rs` is the call site, not the
source.

`mode` in a result now says how the answer was settled, so the composer's mode - a fact about the
request, and always `search` - moves to `composer_mode`. `settled` is `false` only when a caller's
observer stopped the call before the page did; a call that runs out of ceiling fails instead, with
`settled: false` in its detail.

`StableReads` and `Settlement` stay - they are the fallback and the check - and the scroll selector
stays in `Selectors` for sites whose own control is the only way to read a virtualised thread. It
stops being the default path, and no expression presses it: the viewport belongs to the person.

---

## How this is proven

No test suite, no contract script. Two measurements:

1. `cargo build --release -p agito` succeeds.
2. A live run against a bound tab: capture the answer container's `scrollTop` and the window's
   `scrollY` before the call, run `agito chat`, capture them again. Identical before and after is
   the proof that the driver never touched the viewport; a growing `chars` in the signal log is the
   proof that the quiet timer fired at the end of the answer rather than at a poll tick.
