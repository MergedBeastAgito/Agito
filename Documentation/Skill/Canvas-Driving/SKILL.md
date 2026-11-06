---
name: canvas-driving
description: Use when driving a canvas app over CDP. Trusted input, real state, a closed loop.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, rust, canvas, webgpu, input, visual-servoing, control-loop]
        related_skills: [browser-connect, browser-eval, bounded-calls]
---

# Canvas Driving

A canvas app answers no DOM question: the page is a shell, the state is in memory or in pixels, and
an input that failed to land looks exactly like one that landed. This skill is the loop that drives
such an app: find the browser a person is watching, send input the app will trust, read state from
the cheapest source that exists, and prove motion by measurement rather than by having sent a key.

It applies when the task is "play, steer or operate this" rather than "read this", and when the
client can evaluate but has no input verb: `agito browse` sends one expression, so keys go through
the protocol.

**When a person is watching, order the work by what they can see.** Attach, get one tick that
visibly moves the app, install the overlay, and only then verify; a verification script written
first is time the watcher spends looking at a frozen screen. Verify with the cheapest read-back that
can fail, one `innerText` evaluate.

---

## Procedure

1. **Find the real browser, not the first port.**
   `curl -s http://127.0.0.1:<debug-port>/json/version` and `.../json/list`. A stale headless
   instance and the person's live browser can both be listening, and the version string tells them
   apart (`Chrome/156` against `HeadlessChrome/153`). **Bind by URL, never by a remembered target
   id**: an id dies with the tab's reload and surfaces as a `TypeError` on the websocket
   constructor, not as "no tab". Re-list and match a URL prefix.

2. **Send input the app will trust.** A JS-dispatched `KeyboardEvent` is untrusted, and an app that
   cares marks it: one folds `isTrusted ? 0 : 32` into its modifier bits, so a synthetic key is
   silently the "not a person" variant. Use the protocol, `keyDown` and `keyUp` in pairs, `text`
   only for printable keys, and one value for `windowsVirtualKeyCode` and `nativeVirtualKeyCode`:

    ```js
    Input.dispatchKeyEvent({ type: "keyDown", key: "ArrowUp", code: "ArrowUp" });
    ```

    Prove delivery before trusting a driver: a capture-phase listener recording `{code, isTrusted}`
    reads `true` for a CDP key where an injected `new KeyboardEvent` reads `false`.

    **If the app exposes its own input API** (`session.input.key(code, down)`), that is the cheapest
    path, since an evaluate-only client needs no protocol input verb. But one call is a TAP, as the
    input ring consumes one frame: `setInterval(() => input.key(code, true), 16)` moved 11 m in 4 s
    where a single call moved 0.22 m. On release, clear the interval AND send `key(code, false)`.

3. **Establish the state source before inventing one.** In order: **the app's own source**, if it is
   open, which names the memory map (`#define G_`, `#define CAR_`, constants like `0x00d00000`) that
   a `DataView` over the app's `WebAssembly.Memory` buffer then reads. That beats scanning a
   gigabyte for a coordinate: one port's `runtime/roam_game.c` names the player at `0x00d00000`, the
   car at `[player + 4]`, position at `car + 0x60`, heading at `+0x40c` (u16, 65536 a turn), speed
   at `+0x430`. Then **an exposed JS API**, then **pixels**.

    Input counters (`keysHeld`, `events`, `dropped`) answer "did that key land" better than frame
    diffs: `dropped: 0` with `keysHeld` flipping 1 to 0 is proof of delivery. A canvas app that
    "exposes no state" is a claim about its JS surface, not its memory, and once the source names
    the addresses the discovery probes go too: two ways to read one value can disagree.

4. **Stream frames; do not screenshot per tick.** A WebGPU or WebGL canvas read back through
   `drawImage` + `getImageData` returns black, at mean brightness ~0.03 on a rendered scene, because
   the presented texture is not preserved for 2D readback. `Page.captureScreenshot` works but forces
   a readback on every call and **visibly flashes the canvas**. Use the screencast, with
   `everyNthFrame` 1: ~90 fps at ~35 KB a frame, no blink.

    ```js
    Page.startScreencast({ format: "jpeg", quality: 55, maxWidth: 1058, maxHeight: 470 });
    // on Page.screencastFrame: keep params.data, then ack it or the stream stalls
    Page.screencastFrameAck({ sessionId: params.sessionId });
    ```

    Read the newest frame and analyse that; stop the screencast on exit beside releasing keys.

5. **Pick ONE cue and prove it is world-anchored.** Mask by colour, cluster the mask, discard the
   impostors: a fixed HUD element sits in the same corner every tick, and neon or signage hugs the
   frame edge. Keep clusters above a minimum pixel count, inside the road view, away from the
   corners, and prefer the topmost. Then steer hard and check the cue MOVES: a cue pinned at the
   same x is a HUD glyph, and steering at it drives at the dashboard.

6. **Close the loop with a deadzone, hysteresis and a cap.** Proportional steering on `cue.x - 0.5`:
   a deadzone (~0.05) stops it chasing jitter, a hysteresis band (~0.12) stops it flipping sides for
   a wobble that small, a capped duration (~600 ms) stops a far-off cue spinning the car. Log every
   decision with its reason, so a run can be read back instead of watched.

    **Split the loop by timescale and let the page own the keys.** Four keypresses per tick make the
    tick rate the control rate, so every slow round trip becomes a stutter. Write a _goal_ instead,
    target speed and steering bias, once a tick, and let a page-side interval turn it into keys
    every 16 ms: throttle held under the target inside a deadband, bias pulsed on a duty cycle. At a
    45 km/h target, per-tick keypresses held 44-46 km/h and the goal loop held 42-44. A manoeuvre is
    the exception, since an escape is a _sequence_ of keys: it stops the goal loop and holds keys
    directly, and the release path must stop that loop too.

7. **Measure progress from position, not from the app's speed.** The speed in memory is the
   _wheels'_ speed: a car held against a wall at full throttle still reports 40-odd km/h, so no read
   of it separates driving from spinning, and a rule that trusts it never fires on the car that
   needs it. Keep the previous position, accumulate distance over a window of ticks (~6), and derive
   the verdict from metres per window: at 40 km/h a car covers ~11 m, a car scraping a wall under
   one. Three ticks of "throttle held, window under the floor" is `blocked`, the same before the
   third is `creeping`, under 2 km/h for three is `stalled`. An incomplete window is evidence of
   neither, so presume moving.

    Frame differencing is the fallback when no position is readable: two byte-identical frames
    almost never occur in a live 3D scene, so an equality test never fires and a car pinned against
    a wall looks busy forever. Use the mean absolute difference of the bottom band, the road under
    the camera, between consecutive frames, and calibrate both ends first: parked read 0.5-0.8 and
    driving 14-32 on one scene, so "three readings under 5" is a stall.

8. **Recover with a reverse AND a turn, then resume.** On a jam: lift the throttle, reverse _with a
   steering key held_, and when the reverse ends hold that same side with the throttle coming back
   up for a fixed number of ticks (~10) before the ordinary rules resume. Reversing alone leaves the
   car a metre further from the wall and still pointing at it, so the next tick drives back in:
   alternating sides without turning sawed at one corner for 150 ticks, against 5 ticks of reverse
   plus 10 of turn reaching 40 km/h in a new direction. Hold the side that was chosen rather than
   recomputing it from an attempt counter, and fault-inject the branch with an environment override
   that fakes the motion signal, so it is proven on demand.

9. **Inject a HUD for both directions.** A fixed panel showing what was sent (key, duration) AND
   what the app reports (its counters, its stats, the cue position the analyser measured) makes the
   loop legible: read it with one `Runtime.evaluate` of `innerText` instead of decoding a
   screenshot. Label the two directions (`you` / `game`) and push the driver's decisions into it.

    Observations are read by the panel on its own timer; only decisions are pushed. A failed read is
    a state (`reading` / `STALE`), never a silent fallback to old values. The rules that keep a
    panel from rendering stale truth, and the four DOM assertions that prove it updates, are in
    `Reference/Overlay-Injection.md`.

---

## Pitfalls

- **`phase: playing` does not mean a car exists.** The player pointer can still be 0, after a
  reload, a menu or a world not yet spawned, and a chain returning 0 is a state rather than a fault:
  return null, label the panel `STALE`, re-read next tick. "The page threw" usually means the app is
  not running.
- **Never claim the app reacted without a measured signal.** "keysHeld flipped 1 to 0 with dropped
  0" or "the cue moved 0.816 to 0.519 under a hard left" is evidence; "it should be turning now" is
  not. A car at 14 MPH with a frozen cue is parked, however many keys were sent, and a cue that
  never moves means the camera is not moving either.
- **Two clusters, two opinions.** With a minimap and a route arrow both cyan, the largest-cluster
  rule alternates between them and the car saws left and right. Cut the HUD region out by geometry
  first.
- **In-page `requestAnimationFrame` polling is not perception.** It reads the app's JS state, never
  its pixels; anything visual needs the compositor.
- **A verification run collides with a live driver.** A probe that drives the same app fights the
  long-lived run for input, and a shared trace it truncates destroys the evidence it was meant to
  check.
- **A live loop keeps its state in memory, not in a file.** A bounded in-page table with `publish`,
  `subscribe` and `latest` (a few hundred rows per topic) lets the overlay, a second driver and an
  adapter read the same state as it happens; two writers truncating one `.jsonl` destroy each
  other's evidence. Keep the ring bounded.
- **A car that stops when a tick-bounded run ends is a release, not a wall.** The release path lifts
  every key by design, so the car coasts and halts. Say which one with counts: verdicts from the
  trace (`moving` 1454, `creeping` 2, zero `blocked`), the ~28 m from the last position to the
  parked one after releasing at 42 km/h, and `keysHeld: 0`. Lift every key on completion, on Ctrl-C
  and on error, or a held throttle outlives the script.
- **Never call `Page.bringToFront`.** It raises the tab and steals the person's focus on every tick.
  CDP key events reach the page without it, and a page-side interval needs no focus at all.
- **Three traps belong to reading the run, not to driving it.** A perfect correlation on a monotone
  series, a cloned deadline that expires mid-run, and a tree a background worker is editing all
  present as defects in the loop; `Reference/Loop-Traps.md` has the tests that separate them.
- **Pulse the steering, hold the throttle.** Duty-cycled steering inside a fixed 110 ms window
  removes the visible jerk, but duty-cycling the throttle the same way lost thrust and stalled the
  car nine times a minute: throttle wants hysteresis, on below `target - 1`, off above
  `target + 1.5`.
- **Other objects are the app's own list, and their fields are only valid for live ones.** Walk the
  array or list from the app's head pointer and classify each entry by its own type field. Gate
  every motion field on that type: a pooled object's velocity words hold whatever the allocator
  left, and read as motion they put 220 and 361 km/h on cars that had not moved a metre.
  Position-derived distance and bearing stay valid for every entry.
- **A loop with no lateral observation does not steer; it holds a heading until geometry
  intervenes.** Over five minutes, 1,416 of 1,497 ticks read `no cue`, `cruise` was the action on
  1,383 of them, and all 114 lateral ticks (48 steer, 33 brake, 33 reverse) landed after a
  collision. The remedy is a new state source, not a different number.
- **Count verdicts with the app's whole vocabulary.** A tally written from the words you recall
  (`moving|blocked|jammed`) drops the ones you did not (`stalled`, `creeping`) and turns a run with
  two collisions into "1,383 ticks, all moving". Read the vocabulary out of the code that emits it.
- **A model asked on a cadence re-answers on a cadence.** An obeyed "reverse" that _restarts_ a
  manoeuvre is a car that never turns out: 300 ticks of reverse, 262 stalled, 204 m travelled,
  against 1004 m once an answer could choose only the side. Only the rules may start one.
- **Three more traps belong to reading the run, not to driving it.** A perfect correlation on a
  monotone series, a cloned deadline that expires mid-run, and a tree a background worker is editing
  all present as defects in the loop. `Reference/Loop-Traps.md` has the tests that separate them.

---

## Reference

`Reference/Index.md` - the file list for this skill.

---

## Claims and how to test them

| Claim                              | Probe                                                     | Pass condition                                                    |
| ---------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------------- |
| CDP input is trusted               | a capture-phase listener records `isTrusted`              | CDP keys `true`, an injected `new KeyboardEvent` `false`          |
| Input reached the app              | read the app's `status()` before, during and after a hold | `keysHeld` 1 to 0, `dropped` 0                                    |
| The state source is real           | read the pointer chain, then drive                        | the car is non-zero, position changes, speed rises under throttle |
| Frames are cheap enough to loop    | average streamed frame size                               | tens of KB, not megabytes                                         |
| The cue is world-anchored          | steer hard, sample the cue's x                            | cue x moves; a fixed value is a HUD glyph                         |
| The car is moving                  | bottom-band temporal diff, or the app's own speed         | above the calibrated driving floor                                |
| Stall recovery works               | fake the motion signal with the environment override      | reverse fires, the reason is logged, every key is released        |
| The overlay does not flash         | marker property on a child, read after a second           | the marker survives, so the node was never rebuilt                |
| A candidate signal is real         | correlate, then read it parked against driving            | moves while driving, holds while parked                           |
| The app's speed cannot say "stuck" | hold the throttle against a wall at a fixed position      | the window goes to 0 m and the verdict is `blocked` at window + 3 |
| A moving car is never flagged      | drive 2.5 m a tick for 24 ticks                           | the verdict stays `moving`                                        |
| The escape actually escapes        | trace a jammed run end to end                             | reverse for N ticks, a held turn, then speed in a new direction   |
| The loop's budget is per operation | run past the span                                         | ticks keep succeeding after the first span elapses                |
| Another object's speed is real     | read it while that object is parked                       | reports 0, not leftover bytes                                     |
| A cruise tick writes a goal        | send one tick, read the expressions the page received     | one `cruise(on, kmh, bias)`, no per-key holds                     |
