# Worked instance: a WebGPU racing port

A WebGPU recompilation of a racing game, served as a static page, used to work out the
canvas-driving loop. Everything below was read off the live page or its source, not inferred from
the site's docs.

## Start here: the source names the addresses

The port is open, and its `runtime/roam_game.c` is a symbol map for the running game: `#define G_*`
for globals, `#define CAR_*` for the car struct's fields.

```sh
grep -n '#define G_\|#define CAR_' runtime/roam_game.c
```

That one command turns "scan 1 GB of memory for a coordinate" into a handful of reads. The same file
shows how the port itself reads the car (`local_state()`), which is the exact sequence to copy.

## Page surface

`window.__app` exposes:

| Member                          | What it gives                                                                           |
| ------------------------------- | --------------------------------------------------------------------------------------- |
| `phase`                         | `"playing"` once the world is running                                                   |
| `session.input.status()`        | `{attached, events, dropped, keysHeld, gameInactive, quitRequested, cursor:[x,y], pad}` |
| `session.input.key(code, down)` | injects a key by DOM `code` (`"ArrowUp"`) straight into the emulator                    |
| `session.stats()`               | `draws`, `passes`, `gpu encode ms`, `lan rtt ms`, `audio underruns`, `input events`     |
| `session.proc.memory`           | a real `WebAssembly.Memory` (1 GB buffer), the game's own RAM                           |
| `session.renderCheck()`         | renderer state, or `null`                                                               |
| `checks()`                      | browser, GPU and module readiness records                                               |
| `log.lines`                     | the game's own log: free-roam joins and leaves, LAN stats                               |

The JS surface has no position, heading or speed. **The memory does.**

## Memory map (WASM build)

| What                            | Where                                            |
| ------------------------------- | ------------------------------------------------ |
| `PlayersByNumber[0]`            | `0x00d00000` (WASM build; `0x008900ac` native)   |
| car                             | `[player + 4]`                                   |
| position, m                     | `car + 0x60` float[3]                            |
| velocity                        | `car + 0x70` float[3]                            |
| basis vectors (forward/left/up) | `car + 0x90`, `+0xa0`, `+0xb0`                   |
| heading                         | `car + 0x40c` u16, 65536 a turn                  |
| speed, signed m/s               | `car + 0x430` (and `+0x42c`)                     |
| rpm / gear                      | `car + 0x440` / `+0x434` i8                      |
| steering angle                  | `car + 0x40e` u16                                |
| race type                       | `0x0089e7b0` u32 (0 free roam, 1 race, 7 outrun) |

Read them with a `DataView` over `session.proc.memory.buffer`, little-endian. A pointer chain that
returns 0 means the world has no car yet - that is `None`, not a car at the origin.

## Input

Arrow keys drive: `ArrowUp` gas, `ArrowDown` brake and reverse, `ArrowLeft`/`ArrowRight` steer. The
page's modifier encoder is `(alt?2:0)|(ctrl?4:0)|(shift?8:0)|(isTrusted?0:32)`, so a synthetic
`KeyboardEvent` is not equivalent to a CDP key press.

`session.input.key(code, true)` is a **tap**: the input ring consumes one frame. Held from a 16 ms
interval it moved the car 11.38 m in 4 s and reached 32 km/h; a single call moved it 0.22 m. It
bypasses the DOM path, so it does **not** increment `events` or `keysHeld` - verify it by position,
not by those counters.

## Perception

- Canvas is `#game`, `webgpu`, backing 1978x1490 (CSS half that).
- `drawImage(canvas)` + `getImageData` returns black (mean brightness 0.03): the presented texture
  is not preserved for 2D readback.
- `Page.captureScreenshot` works but **flashes the canvas** at a few ticks a second, because every
  call forces a compositor readback.
- `Page.startScreencast({format:"jpeg",quality:55,maxWidth:1058,maxHeight:470})` pushed 135 frames
  in 1.5 s at ~35 KB each, no flash. Ack every frame.

## Injected UI

- `<style>` elements do not apply on this page (CSP): a probe rule computed `position:static` and
  `document.styleSheets` parsed zero rules from 1,538 characters of valid CSS. `el.style.cssText`
  works - style every element inline.
- Rebuilding `innerHTML` per tick flashes. Build once, write value nodes only, cache the last
  string, repaint on a 250 ms timer.
- `pointer-events:none` on the panel with `auto` on the buttons lets clicks pass through to the
  canvas; `document.elementFromPoint` over the panel body returns `game`.

## The route cue

The world route arrow is a saturated cyan glow: `b >= 150`, `b - r >= 45`, `g >= 90`. Two impostors
sit in the same frame:

- the speedometer's nitrous ring in the bottom-right, drawn cyan and never moving with the car
  (roughly `cx > 0.72 && cy > 0.62`) - the actual minimap is a circle in the bottom-**left**, with
  orange roads;
- neon and shop signs hugging the frame edge, which produced a false positive at `cx 0.015` that
  steered the car hard left into a wall.

Filter: keep clusters with `pixels >= 120`, `0.05 < cx < 0.95`, `cy < 0.75`, and outside the corner;
of those, take the smallest `cy` (furthest ahead).

Proving the cue is world-anchored: a hard left turn moved it from `x 0.816` to `x 0.519`. A cue
pinned at the same x is a HUD glyph.

## Motion signal

Mean absolute difference of the bottom 28% band between consecutive frames, on greyscale at 247x186:

| State                       | Reading                                  |
| --------------------------- | ---------------------------------------- |
| parked (no input, 6 frames) | 0.52, 0.73, 0.71, 0.79, one 7.89 outlier |
| driving (40 ticks)          | min 8.9, max 85.0, mean 34.6             |

Threshold: `band < 5` for three consecutive ticks is a stall. Byte-identical frames never fired.
With the memory map available, prefer `speed < 1 m/s for 3 ticks` - it is exact, and the band is
only the fallback when no pointer resolves.

## Correlation trap, with the numbers

Searching memory for a "position" by correlation produced `r = 1.0` on two offsets reading
`0.06, 0.13, 0.50, 0.99` and `0.74, 0.46, 0, -1` - monotone series tracking a monotone cue. Making
the cue non-monotonic by alternating the steering produced coordinate-looking candidates
(`179.63, -15.99, -16.55, -19.51`), which then moved 71-250 units while the car was **parked**. A
position holds still when parked; nothing found this way survived that test.

## Loop parameters that behaved

- deadzone 0.05 around centre, hysteresis 0.12 before reversing steering direction, steer
  `clamp(|err| * 1400, 120, 600)` ms;
- smoothness comes from duty-cycled steering in a fixed 110 ms window at a 150 ms tick, not from
  longer holds;
- throttle wants hysteresis, not PWM: on below `target - 1`, off above `target + 1.5`, hold in
  between (pulsing it stalled the car nine times a minute);
- recovery: lift the gas, reverse 900 ms while steering off the wall, resume;
- a 60-tick run at 150 ms takes ~10 s and covers ~300 m.
