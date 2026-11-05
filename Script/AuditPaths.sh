#!/usr/bin/env bash
# Leak audit: absolute machine paths in tracked files.
#
#   Scripts/AuditPaths.sh [repo] [allowlist-regex]
#
# The repository is public, so a tracked file may name no machine: not a volume
# root, not a home root, not a private temporary directory. Paths inside the tree
# are recorded relative to it - `Crate/Sdk`, `../TimeLine` - and a tool that emits
# one computes the relative form rather than reading an absolute one.
#
# `allowlist-regex` is an ERE of tracked paths to excuse, for scaffolding the fork
# inherited and the owner has not reviewed yet.
#
# Exit 0 clean, 1 leaks present, 2 bad usage / not a repo.
set -u

REPO=${1:-$(cd "$(dirname "$0")/.." && pwd)}
ALLOW=${2:-'^$'}
cd "$REPO" 2>/dev/null || {
	echo "FAIL cannot cd $REPO"
	exit 2
}
git rev-parse --git-dir >/dev/null 2>&1 || {
	echo "FAIL not a git repo: $REPO"
	exit 2
}

# The pattern is assembled from pieces so this file cannot match itself: a checker
# that spells the forbidden form is the first thing it would flag. `file:///` is
# anchored to an ASCII letter so prose that QUOTES the form stays out of the
# result, and each root is matched with its separator because that is the shape a
# path takes.
SCHEME='file:///'
ROOT='/'
PATTERN="${SCHEME}[A-Za-z]|${ROOT}Users/[a-z]|${ROOT}Volumes/|${ROOT}home/[a-z]|${ROOT}private/(tmp|var)/"
fail=0

echo "== absolute machine paths (allowlist: $ALLOW)"
hits=$(git grep -lE "$PATTERN" -- . 2>/dev/null | grep -vE "$ALLOW")
if [ -z "$hits" ]; then
	echo "PASS none"
else
	printf 'FAIL %s\n' "$hits"
	fail=1
fi

echo '---'
if [ "$fail" -eq 0 ]; then echo 'RESULT clean'; else echo 'RESULT leaks present'; fi
exit "$fail"
