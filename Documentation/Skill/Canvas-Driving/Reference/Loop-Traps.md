# Traps in a run's evidence

Four failures that look like defects in the driving loop and are really defects in how the run is
read. Each one produced a wrong conclusion before it produced a rule.

## A perfect correlation on a monotone series is not evidence

Searching memory for a "position" by correlation produced `r = 1.0` on two offsets reading
`0.06, 0.13, 0.50, 0.99` and `0.74, 0.46, 0, -1`: eight samples of a steadily drifting cue,
correlated against monotone counters and against coordinate-looking values. Two tests kill it.

- Make the signal non-monotonic by alternating the steering, then re-correlate. The candidates that
  survive are the ones worth testing (`179.63, -15.99, -16.55, -19.51` did).
- Read the candidates parked against driving. A position holds still while the car is parked, and
  those moved 71-250 units with no input at all.

A correlation over a monotone series measures the drift both series share, not a relationship.

## Count verdicts with the app's whole vocabulary

A tally written from the words you happen to recall (`moving|blocked|jammed`) silently drops the
ones you did not (`stalled`, `creeping`) and turns a run containing two collisions into "1,383
ticks, all moving". Read the vocabulary out of the code that emits it, match the trace against the
full set, and report per-word counts. A histogram with a word missing is a wrong answer that looks
like a clean run, and it will be quoted back at you.

## A cloned deadline is a run-length budget

A deadline is fixed when the call is constructed, and a clone carries that instant: a loop that
builds one budget and clones it per tick has a run-length budget wearing a per-tick name. Once it
expires, every later call fails instantly in the provider's vocabulary ("the page never answered")
against a page that answers in a millisecond. Build a fresh call inside the loop.

Three signs say the failure is arithmetic rather than a slow page: it lands at a fixed wall time
from process start, a slower loop reaches it at a lower operation count, and the last successful
operation shows no latency ramp. Before debugging the driver, prove the page innocent from a second
client: one evaluate a second from a throwaway websocket to the same page reads 0-1 ms, which ends
the argument. Without that line every symptom reads as "the page is slow".

## A tree another agent owns cannot be verified by grepping it

When work is dispatched to background workers, they edit the same crate mid-run: a harness that
greps a file fails with "No such file" because the file was deleted and re-added between two
commands, which looks like a code defect and is not. Check whether a worker holds the tree before
verifying it, and scope verification to files you own.

The same hazard applies to editing. A sibling that rewrites a file you are patching silently reverts
your change, and the tool's "modified by sibling subagent but this agent never read it" warning is
the only signal. After such a warning, re-read the file and confirm your change is still in it
before building: a build failure hours later is the cheap version of this, and a silently reverted
fix that still compiles is the expensive one.
