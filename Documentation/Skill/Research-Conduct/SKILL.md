---
name: research-conduct
description: Use when a research answer must be weighed, not trusted.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [research, judgement, verification, prompting, chat-ui]
---

# Research Conduct

How to ask a research model a question, and how to weigh what comes back. These are rules about the
answer, not about the transport: nothing below depends on whether the text arrived from a chat page,
a command-line tool or an API.

What it is not: a prompt-format recipe. A shape imposed on a research answer is a shape the answer
will fill whether or not it had anything to say, which is why the sections below ask for judgement
rather than for a schema.

---

## Over-supply, under-read

Send the real inventory - names, paths, counts, the whole list - and read only the part where the
verdict lands. The remote model's context is cheap; yours is not. A question that withholds the
inventory to keep the prompt short gets an answer about the shape of the problem instead of about
the problem.

Under-reading is the other half. The value of a long answer is usually one paragraph, and a reader
who summarises the whole thing has paid for it twice. Skim for the verdict, then read that paragraph
closely and leave the rest.

---

## A long answer is not a truncated one

Length is evidence in neither direction. A truncated answer stops mid-sentence or mid-table and its
structure does not close; a long answer closes and keeps going.

The tell is the text, not a control offering to show more. That control is a display clamp over text
the page already holds, so pressing it says nothing about what the model produced - and treating a
complete answer as partial costs a second turn to get a worse version of the first.

---

## Ask for the answer, not a format

A research question wants an answer; a format is a guess about what the answer will contain. Impose
a shape only for status you will act on mechanically - a yes or no you branch on, a list of files
you will open. Everywhere else the demanded shape suppresses the disagreement that made the question
worth asking, because a model asked to fill `risk: <one line>` writes one line even when the risk
needs three.

---

## An answer is a hypothesis until the line it names is read

A research model sees the web and other repositories. It does not see the working tree, the
uncommitted diff, or the commit that has not been pushed, so an answer about the code is an answer
about code that resembles it.

Every answer is therefore a hypothesis until the line it names has been opened: the file exists, the
function has that signature, the flag is spelled that way. An answer that cannot be reduced to a
line to open is not yet usable, and the remedy is a narrower question rather than more trust.

---

## Make it argue against itself

Paste the artefact rather than a description of it. A description is already a reading of the
artefact, and the model will agree with that reading because it is the only thing in front of it.

Then ask for the strongest case against its own advice, and ask where the design is worse. A
concession with no downside is a sales pitch: an answer that finds nothing wrong with what it has
just recommended has not looked, or was asked in a way that made looking rude.

---

## One question per thread

A follow-up in an existing thread carries that thread's history, so it does not have to restate
facts a fresh thread would need. That is the whole benefit and its whole cost: the answer budget is
shared with everything already in the thread, so a second question dropped into a saturated thread
comes back truncated - and truncated in a way that reads like a short answer.

Start a new thread when the subject changes. Keep the thread when the question is a correction of
the last one, because the history is what makes the correction intelligible.

---

## Say which model, which mode, which thread

Report all three. A reader who cannot tell which of them produced an answer cannot weigh it, and the
same question in another mode, on another model or in another thread is another answer - a note that
omits them records an opinion as a fact.

---

## What this skill does not cover

Driving a page: the mode and model preconditions, typing as a real edit, the read-back guard, and
settlement as the page's own signal plus stable text. Those are properties of one product's markup
and live in `Chat-Research/`, because this skill is not about any product.

---

## Claim → test

| Claim                                          | Test                                                                                                                                                     |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The inventory goes in, the verdict comes out   | ask a question naming twenty files and asking which one owns a symbol: the answer names one file, and that paragraph alone decides the question          |
| A closed answer is not re-asked                | an answer whose last sentence and last table row are complete is accepted; an answer that stops mid-clause is asked again in a new thread                |
| A demanded shape hides a caveat                | ask the same question with and without a one-line format: the formatted answer drops a caveat the free answer states                                     |
| An answer about the tree is checked against it | open the file and line the answer names: the symbol is there, or the answer is wrong                                                                     |
| Self-criticism is asked for, not hoped for     | ask for the strongest case against the advice: the answer names a cost it had not named before                                                           |
| A saturated thread truncates                   | ask a second unrelated question in a long thread: the answer is shorter than the first and ends mid-clause, and the same question in a new thread closes |
