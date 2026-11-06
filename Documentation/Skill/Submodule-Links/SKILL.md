---
name: submodule-links
description: Use when a doc links a submodule. Name its own repository, not a path under the parent.
version: 1.0.0
license: MIT
platforms: [linux, macos, windows]
metadata:
    hermes:
        tags: [github, submodule, links, markdown, 404, documentation]
        related_skills: []
---

# Submodule Links

A submodule is a separate repository, so a path link into the parent is a 404 - the file exists in
the checkout and the link still fails, which is the defect this skill prevents.
`Documentation/Style.md` states the same rule for this tree; this is the sweep that finds the
violations and the rewrite that fixes them.

## When to Use

- A document links a directory that `.gitmodules` lists as a submodule.
- A link 404s on GitHub while the file exists in a checked-out submodule.
- A sweep of a repository's prose for links, before publishing or after a merge.
- Not for links to ordinary directories in the same repository: those resolve and must be left
  alone.

## The rule

A link to a submodule's content names the submodule's own repository. It is not the parent's HTML
URL plus the submodule's path, and it is not `tree/<branch>/<path>` or `blob/<branch>/<path>` under
the parent: GitHub has no folder there to open, so every such link 404s whatever the branch is
called. `tree/Current` is not special - `tree/main`, `tree/master` and every other name fail the
same way, because the failure is the path, not the branch.

The probe, not an opinion:

```sh
curl -s -o /dev/null -w '%{http_code}\n' https://github.com/MergedBeastAgito/Use/tree/Current/NFSU2   # 404
curl -s -o /dev/null -w '%{http_code}\n' https://github.com/MergedBeastAgito/NFSU2-Use                # 200
```

## Resolve the target from `.gitmodules`

Never derive the submodule's URL from its name: the path and the repository name differ -
`path = NFSU2` has `url = ssh://git@github.com/MergedBeastAgito/NFSU2-Use.git`. Read the owning
repository's `.gitmodules`, from the root of the parent repository:

```sh
git config -f .gitmodules --get-regexp '^submodule\..*\.(path|url)$'
```

Convert the SSH remote to its HTML form: strip `ssh://git@`, strip a trailing `.git`. A submodule of
a submodule resolves against that submodule's own `.gitmodules`, not the root's - `Use/NFSU2` is a
repository with a `.gitmodules` of its own, and its entries are `Use/NFSU2/.gitmodules`.

## Sweep

Run from the parent repository's root:

```sh
grep -rnoE 'https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/(tree|blob)/[^)"]+' \
	--include=*.md --include=*.json --include=*.toml --include=*.yml .
```

Read each hit against the `.gitmodules` map. A hit is a defect only when the segment after
`(tree|blob)/<branch>/` is a submodule path. A path that is an ordinary directory is a correct link
and is left alone - rewriting it would break a link that works.

The sweep is the check. Run it over the tree, not over the file you were editing: the same paragraph
is copied into a README, a package manifest and a workflow, and the one that 404s is whichever was
not looked at. The one expected hit is the probe in this skill's own code block, which is a `404` on
purpose.

## Rewrite

- Submodule root -> the submodule's HTML URL: `https://github.com/<owner>/<repo>`.
- A file inside a submodule -> `https://github.com/<owner>/<repo>/blob/<branch>/<path-inside>`.
- A link whose text names the parent's path (`Use/NFSU2`) keeps that text; only the target changes,
  and the sentence around it must still be true.

## Boundaries

- Third-party submodules are upstream checkouts: their own READMEs are not ours to edit, and a link
  from our docs to them still names their repository.
- No verification script and no CI gate: the sweep plus a manual read is the check. A guard living
  in the repository is the layer these trees remove.
- Prettier formats Markdown: `node node_modules/prettier/bin/prettier.cjs --check <files>` from the
  repository root, or the edit is not done.
- Do not commit unless asked; leave the working tree dirty.

## Reference

`Reference/Index.md` - the file list for this skill.

`Reference/Merged-Beast-Agito.md` - the map from every submodule path in this tree to its
repository, and the one rewrite that produced the rule.

## Claims and how to test them

| Claim                                    | Test                                                                             |
| ---------------------------------------- | -------------------------------------------------------------------------------- |
| A submodule path under the parent 404s   | `curl -s -o /dev/null -w '%{http_code}\n' <url>` -> `404`                        |
| The submodule's own repository resolves  | the same probe -> `200`                                                          |
| The path and the repository name differ  | `git config -f .gitmodules --get-regexp '^submodule\..*\.(path\|url)$'`          |
| No such link remains                     | the sweep above returns nothing                                                  |
| The rewrite did not touch a working link | every hit was a submodule path, and every ordinary-directory link is unchanged   |
| The prose still reads                    | the link text and the sentence around it are still true after the target changed |
