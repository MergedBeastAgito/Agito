# Merged Beast Agito: the map and the one rewrite

This tree's parent repository holds `Agito` and `Use` as submodules, plus `.github`. Each entry in
the parent's `.gitmodules` also carries `ignore = all`, so a change inside a submodule never appears
in the parent's status - which is why a stale link in a submodule's prose is found by a sweep rather
than by noticing it in a diff.

## The map, from the repositories' own `.gitmodules`

The parent repository:

- `Agito` -> `https://github.com/MergedBeastAgito/Agito`
- `Use` -> `https://github.com/MergedBeastAgito/Use`
- `.github` -> `https://github.com/MergedBeastAgito/.github`

`Use`:

- `NFSU2` -> `https://github.com/MergedBeastAgito/NFSU2-Use`
- `Stem/NFSU2Recomp` -> `https://github.com/TalhaMudo/NFSU2Recomp`
- `Stem/N64Recomp` -> `https://github.com/N64Recomp/N64Recomp`
- `Stem/XenonRecomp` -> `https://github.com/hedge-dev/XenonRecomp`
- `Stem/nfsu2-web` -> `https://github.com/th0mk/nfsu2-web`

`Agito` and `Use/NFSU2` carry no submodules of their own.

## The rewrite

`Agito/README.md`, under "Embed it", pointed the worked example at the parent's path:

```
https://github.com/MergedBeastAgito/Use/tree/Current/NFSU2
```

It answers `404`. The submodule's own repository, which answers `200`, is:

```
https://github.com/MergedBeastAgito/NFSU2-Use
```

Nothing else in the tree held the defect: the sweep returned this one line and nothing more. The
`Stem/*` checkouts carry `tree/<sha>` links of their own, which are upstream files and stay as they
are.

## Why the name does not follow the path

The submodule's path is `NFSU2`; its repository is `NFSU2-Use`. Deriving one from the other produces
`MergedBeastAgito/NFSU2`, which is a different project. `.gitmodules` is the only statement of the
mapping, and it is why the rule in `SKILL.md` starts with `git config -f .gitmodules` rather than
with a guess about how the tree names things.
