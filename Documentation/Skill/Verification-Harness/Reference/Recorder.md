# The recorder: verifying a tool whose product is a command

A tool that writes to a network API, a repository or a database has no local product to inspect. What
it produced is the set of commands it issued, and the cheapest way to assert on that is to replace
the runner with a recorder: a function that appends the `argv` it was handed and returns the fixture
that command would have printed.

```python
Issued = []


def Fake(Command):
    Issued.append(Command)
    return FixtureFor(Command)  # the JSON this command would have printed


Module.Run = Fake
```

Nothing else in the tool changes. The tool's own decision logic runs unmodified, and every assertion
becomes a statement about a command that was or was not issued - with no credentials, no rate limit,
no cleanup, and a refusal provable as directly as a write.

---

## The four rules

**Assert the writes you expect and the writes you do not.** "A dropped item is closed and its body is
not edited" is two assertions, and only a recorder can make the second one. A harness that checks
only the first passes over a tool that closes every item it touches.

**Drive the entry point twice.** The first pass must issue the writes the fixture needs; the second,
over a fixture that now looks repaired, must issue none. That pair is the only evidence a repair
finished rather than looping, and it is the reason the fixture is stateful rather than constant.

**The fixture is the other half of the test.** When a new check fires on the fixture that was called
clean, settle the rule in words before bending the fixture. A rule that demands presence fires on
every correct record; a rule that demands agreement fires only on a real error. Bending the fixture
to silence the check is how a harness ends up asserting the tool's current behaviour instead of the
requirement.

**Route the fixture by command shape, not by position.** `Command[3]` breaks the first time an
argument is added. Match the subcommand and the flag that carries the meaning:

```python
def FixtureFor(Command):
    Verb = Command[1]
    if Verb == "list":
        return Fixture["List"]
    if Verb == "close":
        return Fixture["Closed"][Command[Command.index("--id") + 1]]
    raise AssertionError(f"the tool issued a command the harness does not model: {Command}")
```

The `raise` is deliberate: an unmodelled command is a gap in the harness, and it should stop the run
rather than be answered with an empty fixture that makes the tool look idle.

---

## What it cannot prove

The recorder proves what the tool decided, not what the service did with it. A schema the service
would reject, a permission it would refuse, a rate limit it would impose - all of those are outside
the recording. Say so in the report, and run one real invocation against the live service when the
claim depends on the service's answer.
