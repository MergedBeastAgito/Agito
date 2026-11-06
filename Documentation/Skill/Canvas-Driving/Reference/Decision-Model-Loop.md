# Wiring a decision model into a driving loop

A driving loop that can consult a model needs four things the pixels never demand: a request shape,
a probability to gate on, switches a person can reach, and a view that shows the round trip. This is
the recipe, independent of which model or vendor.

## The request is a decision call, not a chat call

Decision models take a **state** and a schema of **typed questions** and return a probability for
every allowed option. That is not the chat-completions shape, and the chat route rejects it - a 400
saying `required properties at '/' are 'model,state,questions'` is the model telling you the body
itself is the schema.

```sh
POST https://api.cloudflare.com/client/v4/accounts/$ACCOUNT/ai/run/@cf/cloudflare/clef-flash
{
  "model": "clef-flash",                 # BARE name; the URL carries the full id
  "state": {"speed_kmh": 40, "gear": 3, "heading_degrees": 355, "route_cue_x": 0.50},
  "questions": {
    "action": {"type": "choice", "instructions": "What next?",
                "criteria": {"steer_left": "…", "steer_right": "…", "throttle": "…", "brake": "…", "hold": "…"}},
    "confidence": {"type": "score", "instructions": "How sure?",
                    "criteria": ["guessing", "unsure", "fairly sure", "certain"]}
  }
}
```

Traps that each cost a round trip:

- the body's `model` must match the **bare** alias (`clef`, `clef-flash`); the full
  `@cf/cloudflare/…` id in the body fails a `^(clef|clef-flash)$` test;
- `questions` is an **object of typed questions**, not a list of words - a list fails as a missing
  property;
- the answer lives under `result.answers.<id>`: a `choice` gives `choice` plus `probabilities` per
  option, a `score` gives a weighted `score` and a `legend`.

`state` may be a string or structured data; a question is `noul` (yes or no), `choice` (a map of
option to description) or `score` (an ordered criteria list); 1-64 questions per request. The reply
carries every option's probability:

```json
{
	"result": {
		"answers": {
			"action": {
				"type": "choice",
				"choice": "throttle",
				"probabilities": {
					"steer_left": 0.112,
					"steer_right": 0.1025,
					"throttle": 0.5348,
					"brake": 0.0299,
					"hold": 0.2208
				},
				"confidence": 0.1984
			},
			"confidence": { "type": "score", "score": 2.1214, "legend": { "2": "fairly sure" } }
		},
		"usage": { "input_tokens": 343, "output_tokens": 0 }
	}
}
```

Cost shape: a call measured 343 input tokens and **zero output tokens**, billed only on input -
$0.038/M for `clef-flash` and $0.24/M for `clef`, so one tick of advice is ~$0.000013. That is why
the model is asked on a cadence (`--every N`) and not per tick; output tokens are never billed.

Two 400s identify the family and are worth recognising: a chat-shaped body draws
`required properties at '/' are 'model,state,questions'`, and the full model id in the body draws
`'/model' failed test ^\s*(clef|clef-flash)\s*$` - the URL carries `@cf/cloudflare/...`, the body
carries the bare alias.

## Gate on the probability, keep the rules

The rules engine stays the driver; the model argues. Record the answer with its probability in the
decision's reason, and obey it only above a threshold. Every failure path - timeout, non-200,
unparseable body, missing token - returns nothing and the tick carries on. A car at 40 km/h must not
stop because a network call was slow.

**Choose the threshold from the model's own answer distribution, not from taste.** Read the
probabilities a real run produced and place the gate where it divides that distribution
deliberately. Measured over 24 calls: min 0.43, median 0.565, max 0.65 - so a 0.60 gate sat _above_
the typical answer and discarded 19 of 24, which reads in the trace as "the model is ignored" and is
really the gate sitting in the wrong place. At 0.50 the same run obeyed 16 of 24; a full five-minute
run at 0.50 obeyed 162 of 220 (74%). Collect the numbers first, then pick the cut and say which
quantile it is.

A gate is also the honest place to state the model's authority: at 0.50 the rules and the model
alternate, which is what "agentic" means. A gate so high that nothing passes is a rules-only run
wearing a model flag, and a gate so low that everything passes is a model-only run wearing the
rules.

## What the model is actually good for (measured)

`clef-flash` over a text state, the `track_cue` template, 38 km/h, asked twice with the direction
convention spelled out in the instructions:

| cue x                   | chosen   | steer_left | steer_right | throttle |
| ----------------------- | -------- | ---------- | ----------- | -------- |
| 0.42 (road turns left)  | throttle | 0.32       | 0.27        | 0.36     |
| 0.58 (road turns right) | throttle | 0.27       | 0.23        | 0.44     |

Near-uniform across options, and the left/right ordering is right at one cue and wrong at the other.
A 9B model reading a dozen numbers is a coin flip with a margin: it is an advisor worth recording,
not a controller. Two consequences - gate on the probability and keep the rules driving; and when
the cue is visual, send the image instead of more prose about the pixels, since text alone
measurably does not carry the direction.

## The state is a scene, and the options are the vocabulary

A decision model handed numbers and no description answers as if the world were empty. Measured: a
car held against a wall at full throttle - wheels turning, no progress - was sent `speed_kmh: 46`
and answered `throttle`, which is not a mistake given what it was shown. Two changes fixed it, and
both are needed:

- **Send the verdict and the sentence that explains it**, beside the numbers:

    ```json
    {
    	"verdict": "blocked",
    	"situation": "the throttle is held and the car has advanced only 0.6 m over the last 6 ticks, turning 18 degrees without getting free - it is pressed against something (a wall, a kerb or a pole) and cannot go forward",
    	"speed_kmh": 5,
    	"moved_m": 0.6,
    	"blocked_ticks": 3
    }
    ```

    The numbers say what was measured; the sentence says what they mean. A model cannot disagree
    with a state that contains no claim.

- **Make the escape an option.** A question whose options are
  `steer_left | steer_right | throttle | brake | hold` cannot be answered "reverse out", however
  well the model understands the wall. Switch the option set on the verdict: when jammed, add
  `reverse_left` and `reverse_right` and say in the instructions that reversing with the wheel
  turned is how a car leaves a wall.

Three rules keep an obeyed escape from being undone:

- an obeyed reverse is a **manoeuvre, not a tick** - hold it for several ticks, or a 150 ms tap
  moves nothing and the next tick drives back into the wall;
- while a manoeuvre runs, gate every answer that is not an escape: `throttle` re-enters the wall,
  `steer` abandons the turn carrying the car away;
- refuse a reverse on an open road - `reverse_left` while the verdict is `moving` is a misreading,
  not a decision, and obeying it undoes real progress.

The rules must read the same verdict. A stall rule that tests the reported speed only never fires on
the car at 46 km/h against a wall - the exact car it exists for.

## The question must not contain its own answer

Measured, and it cost a whole question set: with a car under 60 m ahead in lane, `clef-flash`
answered `throttle` on 22 of 31 calls - and it was right. The instructions said ghost mode was on,
so contact could not happen. The framing told the model the traffic was not a hazard, and the model
believed it.

A question that states the consequence in its own instructions has pre-answered itself. Before
adding a question, read the instructions as the model reads them: if the answer follows from the
prose, the numbers are decoration.

- Ask about what is still undecided - the line, the pace, the gap - not about the thing the prose
  already settled.
- If the world genuinely removes the hazard, do not ask a hazard question. Either ask nothing, or
  ask the question that remains ("hold the pace", "move to the left of it").
- The same applies to a `situation` sentence: a sentence that says "nothing is wrong" produces
  `throttle` however alarming the numbers beside it are.

## Log the question, not just the answer

`throttle` at a wall and `throttle` on an empty road are different answers, and a trace that records
only the chosen word cannot tell them apart afterwards. Carry the question's id in the answer
(`asked: open | traffic | blocked`) and print it beside the decision.

Without it a post-run count reads as "the model ignored the traffic" when the traffic question was
never asked - the nearest car in the trace was in another lane, and the driver had correctly asked
the open-road question. Two numbers, not one, or the measurement is of your own bookkeeping.

## Schemas: the ingestion and the question templates

Two files, not prompts built inline:

- an **observation schema** naming every field with its provenance - memory address or captured
  frame - and its range, so a question can never ask about a value the driver does not send;
- a **question template per use case**, each naming the `when` condition it applies to, so the
  driver selects by state instead of guessing which question is current.

Every template is typed questions over those fields with all options written out. A template that
needs a new field means adding it to the observation schema first, with its address: a question
about an unsent field is unanswerable, and the model will still answer it.

## One authority axis, one transport fact

Expose the machinery, because a hosted model is billed per call - but expose it as the questions it
actually answers, not as a row of toggles that happen to be clickable.

- **authority** - one axis, three points: rules only; rules driving with the model arguing (gated);
  the model deciding. The two halves this turns on are _derived_ from it and must be **reported, not
  offered**: `rules only · no request`, `rules + model · gated at 0.50`,
  `model only · rules fill the gaps`. Rendering `LOCAL` and `REMOTE` as separate switches beside
  this axis is the mistake - they are not independent, so a reader who turns one off gets nothing it
  promises.
- **transport** - whether a request leaves at all (`pipe open` / `pipe broken`). Its own labelled
  group: it is a wire fact, never a peer of the authority choice, and with it broken the driver
  still drives while the wire view says nothing was sent rather than implying the model had no
  opinion.

Write them to page globals (`window.__pipe`, `window.__power`) so every driver - JS, Rust, a second
tab - reads the same switch the person clicked, with no message-passing layer. Read them every
paint, not once.

**Only claim a global is shared if the running driver reads it.** A Rust driver that takes `--every`
and `--threshold` as flags reads none of them, so a `TUNING` group of page inputs steers nothing
while looking authoritative. Label it with its real scope (`TUNING · page harness only`) and state
what the process actually reads - a knob that does not steer the running loop is worse than no knob.
Grouping rules for the whole panel: `Overlay-Injection.md`.

## The lane makes the pipeline auditable

A full-width strip with four columns and toggles is what turns "it drives" into "here is what it
sent, got, and did":

| column  | content                                                      |
| ------- | ------------------------------------------------------------ |
| SEND    | the exact request body, built by the same code that sends it |
| RECEIVE | the answer, or an honest reason there is none                |
| APPLIED | the action the driver took, with its duration                |
| TRACE   | the last few ticks, so a run reads as a sequence             |

Two rules that keep it trustworthy: the SEND column must show the **real** body - mirror the sender
exactly, or reuse its builder - and RECEIVE must say `pipe broken` or `user powered` rather than
implying a model was consulted when it was not. A pipeline view that can only show success is
decoration.

### The bodies travel on the payload, not from the harness

A strip fed by the harness proves what the harness _would_ have sent. The real round trip belongs on
the tick payload - `{ send, recv, error, ms, lane }` published beside the decision - so the panel
shows what the driver actually sent and got whether the driver is JS, Rust or a second tab, and a
tick with no request is visibly empty instead of silently reusing the last one.

Pretty-print a body when it parses and show it raw when it does not: a malformed reply rendered as a
blank is a reply nobody can debug. Bound `recv` (~4k characters) so a chatty answer cannot bloat the
per-tick payload.

### Never put a credential on screen

The request body is `{model, state, questions}` and carries no secret; the token lives in a header
and must never enter the payload. Run both bodies through ONE redactor at the display boundary
anyway - `Bearer <…>`, any 20+ character run of hex or alphanumeric, and the words `Authorization`,
`API_TOKEN` and `apiKey` become `***`. Redact where it is drawn rather than trusting the payload's
shape, because the next field added to a request is the one nobody re-checked, and a screenshot of
this panel is what gets pasted into a chat.

### A columnar readout beats a list of rows

Candidates, the duty each would hold, the model's probability and its confidence are four numbers
about the same tick. Put them in four columns with a pipe between every pair and one pipe rule under
the title (`| | | | |`), so the eye compares a candidate's duty against its probability on one line
and sees at a glance which number the gate actually reads. A list of `name … value` rows makes the
reader hold one row at a time and compare from memory, and it hides the fact that probability is the
thing that decides.

Build the columns as a CSS grid with a fixed track per pipe and give every value cell a unique
`id` - see `Overlay-Injection.md` for why the cache helper silently freezes cells otherwise.

## Modes choose the dimensions; the tick rotates the rest

Sending the whole state on every tick is waste, and sending the _same_ shape on every tick is worse:
two identical requests make a model that answers from habit indistinguishable from one that answers
from the state. Compose the body from **dimension blocks**, and let the app decide which blocks a
tick carries.

| dimension | carries                                                         | source                |
| --------- | --------------------------------------------------------------- | --------------------- |
| motion    | speed, gear, rpm, heading, verdict, metres moved, blocked ticks | the app's own struct  |
| traffic   | the lead's gap, speed, closing rate and kind, how many are near | the app's object list |
| route     | the route cue's screen x, track                                 | frame + globals       |
| race      | racetype, career race, flow                                     | globals               |
| wheel     | surface, slip, grounded                                         | the app's own struct  |

`motion` is in every send; the others are chosen. The **mode** is read from the app, never set by a
flag

- a jammed car is recovering whatever the race flags say, a race is racing, and a cue on screen is
  what separates a mission from a cruise:

| mode    | when                            | always                | every 4th tick |
| ------- | ------------------------------- | --------------------- | -------------- |
| cruise  | free roam, moving               | motion + traffic      | route          |
| mission | a route with an arrow on screen | motion + route        | traffic        |
| race    | racetype or career race set     | motion + route + race | traffic        |
| recover | blocked or stalled              | motion                | traffic        |

Two properties fall out, and both are the point. The send **differs between consecutive ticks**
because one dimension arrives on a tick count rather than when a caller remembers it. And the
**question follows the mode**: recover offers `reverse_left`/`reverse_right`, race asks about pace
and adds a risk score, cruise asks about the line and the pace. Put the mode and the focus in the
body (`"mode": "cruise", "focus": "route"`) so a trace can say what a tick was for.

What this is not: a schema with unused fields. A race send does not carry the wheel block, and a
cruise send does not carry `racetype` every tick - a field the question cannot read only costs
tokens.

The field sets themselves, measured with the same question against `clef-flash`:

| lane    | fields                                                              | when                     |
| ------- | ------------------------------------------------------------------- | ------------------------ |
| drive   | speed, gear, cue_x, verdict, situation, moved_m                     | every tick               |
| corner  | speed, heading, cue_x, yaw_rate                                     | approaching a turn       |
| recover | verdict, situation, moved_m, blocked_ticks, gear, grounded, surface | jammed or slipping       |
| race    | speed, gear, racetype, track, cue_x                                 | a race is running        |
| context | the full state                                                      | judgement, never control |

The **sentence is not a dimension optimisation**: it travels with the numbers on every send that
asks for a decision, because dropping it to save tokens is what produces a confident `throttle` at a
wall. What the choice of dimensions saves is the fields a question never reads.

Measured with the same question against `clef-flash`: **375 input tokens for the three-field drive
lane against 499 for the full sixteen**, and latency 451 ms against 268 ms.

Read that second number properly: **lanes buy tokens, not speed.** At these payload sizes the round
trip is network variance, and the smaller body was the slower call. A faster send comes from cadence
(`--every N`) and from answering on the page - never from assuming a shorter body arrives sooner.
Say so, because "smaller request = lower latency" is the intuitive claim and the measurement refutes
it.

## Layout budget

A full-width strip pinned to the bottom occupies its whole height: any panel anchored to
`bottom: 8px` now sits underneath it. Lay panels out by edge budget - HUD top-left, observations
top-right, decisions above the strip (`bottom: <strip height + gap>`) - and assert pairwise
non-overlap rather than looking at it.
