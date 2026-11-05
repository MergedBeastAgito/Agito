---
name: bounded-calls
description: Use when one Call spans a loop, a run, or a sequence of operations.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [rust, agentic, call, deadline, budget, cdp]
---

# Bounded Calls

A `Call` is a deadline and a cancellation, and **the deadline is fixed when the `Call` is built**.
Everything below is what a program does to itself by forgetting that one sentence.

---

## One `Call` per operation, never one per run

```rust
// wrong: the deadline is the run's start plus the span
let Bounded = Call::Within(Span, &Tokio, Arc::new(Never));
loop {
    Port.Call(Request, Bounded.clone()).await?;
}

// right: a fresh deadline for the operation being performed
loop {
    let Bounded = Call::Within(Span, &Tokio, Arc::new(Never));
    Port.Call(Request, Bounded).await?;
}
```

`Call::Within(Span, Clock, Cancellation)` evaluates `Deadline = Clock.Now() + Span` in the
constructor, and a clone carries that instant. A run that builds one `Call` and clones it per tick
therefore has a run-length budget wearing a per-tick name.

**The failure it produces is a lie.** Once the single deadline passes, every later call is already
expired, and the error arrives in the provider's vocabulary - `timeout: evaluate exceeded its
deadline`, "the page never answered" - against a page that answered in a millisecond. The reported
subject is the page; the real subject is the caller's arithmetic.

The three signs that separate this from a slow page:

- the failure arrives at a **fixed wall time from process start**, not at a fixed operation count;
- a slower loop reaches the same failure at a **lower operation count** and the same age;
- the last successful operation is **normal** - no latency ramp, no partial result, no warning.

---

## Prove the page is innocent before blaming it

Two clients, one page. The program under test holds one connection; a throwaway script holds a
second, one line a second:

```js
// node, a plain CDP websocket to the page's own webSocketDebuggerUrl
const Ms = Date.now();
const R = await Send("Runtime.evaluate", { expression: "1+1", returnByValue: true });
console.log(Date.now() - Ms, R.result.result.value);
```

If the second client answers in 0-1 ms while the first client's call never returns, the page is
fine: the difference is in the caller - its deadline, its transport, or its handler task. That
measurement costs one line a second and it ends the argument in one direction or the other.

---

## A handler that ends leaves a connection that lies

`chromiumoxide`'s `Handler` is a stream. When it returns `None`, the connection stays open, accepts
commands, and never answers them; each later call then waits out its own deadline, so a dead socket
is reported as a slow page.

Mark the transport closed from the handler task, and check the flag before every call:

```rust
let Closed = Arc::new(AtomicBool::new(false));
let Mark = Closed.clone();
tokio::spawn(async move {
    let mut Handler = Handler;
    loop {
        match Handler.next().await {
            Some(Ok(())) => {},
            Some(Err(Why)) => eprintln!("unreadable message from the browser: {Why}"),
            None => break,
        }
    }
    eprintln!("the connection to the browser ended");
    Mark.store(true, Ordering::SeqCst);
});
```

A panic inside a spawned task is silent by default: the task dies, the channel it served stays open,
and the caller waits. A panic hook is the cheapest way to tell a crash from a stall:

```rust
std::panic::set_hook(Box::new(|Info| eprintln!("panic: {Info}")));
```

---

## What this is not

- **Not a timeout policy.** The span's value is a separate decision. This skill is about *when* the
  value is computed.
- **Not a retry strategy.** A per-operation deadline makes retries meaningful; it does not add them.
- **Not a claim that every expired call is the caller's fault.** A page that is genuinely busy
  expires too - and it expires in the second client as well, which is the whole reason to run one.

---

## Claim → test

| claim | how it is checked |
| --- | --- |
| a `Call` fixes its deadline at construction | `Call::Within` body: `Deadline: Clock.Now() + Span` |
| a reused `Call` reports the provider's timeout word | run a loop past the span; the error names the provider's phase, not the caller's |
| a second client proves the page alive | two CDP clients: latency of the second while the first hangs |
| a dead handler is silent | the transport's closed flag is set only by the handler task; check it before a call |
