# Before and After

Five pairs, each one an instance of a rule stated in `SKILL.md`. The rule stays in the body; the
rewrite lives here.

## A "not" becomes two sentences

Before:

> The bind must not close the tab on release.

After:

> A bind drives a page the browser already holds. It does not close that page when the call is
> released, because the person was reading it.

The second sentence is not padding: it is the consequence, and the consequence is what a reader
remembers when the first sentence is ambiguous.

## An aside becomes a clause

Before:

> Call `dispose()` - it is asynchronous - and then drop the handle.

After:

> `dispose()` is asynchronous. A caller that drops its future leaves the browser running, so await
> it before releasing the handle.

A parenthetical is a fact the author did not want to interrupt the sentence for. That is exactly the
fact a reader skips, so it gets its own sentence.

## A ceremony becomes steps

Before:

> Carefully attach to the browser and evaluate the expression, making sure the page is ready first.

After:

```sh
curl -s http://127.0.0.1:<debug-port>/json/version
Target/debug/agito browse --attach <ws> --bind <id> --expression 'document.title'
```

Two commands, in order, with the second one naming what the first one produced. "Carefully" and
"making sure" are instructions with no action in them, and a reader cannot run either.

## A five-slot bullet is unpacked

A bullet carrying five facts at once - when to use it, the command, what it returns, what to do when
it fails, and why - is five lines wearing one line's clothes.

Before:

> Use `agito verify --module <id>` to check readiness, it prints a table, and if the manifest
> drifted it says so, because the manifest is a claim about the code.

After:

> `agito verify --module <id>` reports one module's readiness, and whether its manifest still
> matches its code.
>
> ```sh
> agito verify --module @agito/browser-cdp
> ```
>
> A drifted manifest is a claim about the code that the code no longer supports. The check exists
> because the manifest is what a consumer reads instead of the source.

The trigger, the command, the failure and the reason are now four slots a reader can find
independently, and only one of them is prose.

## A scar becomes a test case

Before:

> We once saw a bind hang because the target had not arrived yet.

After:

> A page cannot be handed out until its session arrives, so a first match can legitimately miss. The
> match is retried, and past the grace the failure names every page the browser holds - which is how
> a race is told apart from a wrong id.
>
> | Claim                         | Test                                                                   |
> | ----------------------------- | ---------------------------------------------------------------------- |
> | A wrong target id is reported | `--bind deadbeef` exits non-zero and lists the pages the browser holds |

The narration is gone and the knowledge is not: what was a story is now a mechanism and a probe.
