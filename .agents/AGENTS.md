# Agent entry point

A tool that looks for `.agents/` finds this. The repository's own instructions are one level up in
`AGENTS.md`; this file exists so an agent that starts here is not stranded.

## Where the procedures live

`.agents/skills` is a symlink to `Documentation/Skill/`. The directory is the real one - every skill
carries a `SKILL.md` and, when it has supporting files, a `Reference/Index.md`, and the files a skill
archived sit beside it.

**The symlink is a convenience, and nothing depends on it.** Git stores it as a link, so a checkout
on macOS or Linux gets a working directory there. A checkout with `core.symlinks=false` - the
default on Windows without Developer Mode - writes a small plain file instead, whose entire content
is the target path:

```sh
cat .agents/skills        # ../Documentation/Skill
```

If that is what you find, read the path and go there, or use `Documentation/Skill/` directly. The
skills are the same files either way; only the short path is missing.

Nothing is duplicated here. A second copy of a skill is a second answer to the same question, and the
copy that goes stale is always the one nobody is reading.

## The rule

Read the skill before the task, not after it fails. A skill states the failure it exists to prevent;
a reader who arrives without it repeats the failure and then reads the paragraph that predicted it.

## Which one

The table in `AGENTS.md` at the repository root maps a task to the skill that covers it. It is the
only list - this file does not keep a second one, for the reason above.

## If a skill is wrong

Fix it in place. A skill that documents a command nobody has is a lie, and a skill that omits the
failure it was written for is worse. `agito skills` validates the corpus - a closed frontmatter
block, a `name` matching the directory, every `Reference/` file indexed, every skill named in
`Documentation/Skill/Index.md` - and the tree is public, so a correction here reaches every clone.
