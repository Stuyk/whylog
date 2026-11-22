---
name: whylog
description: Record agent work sessions and lasting decisions with the whylog CLI, and read past decisions before changing a project. Use at the start and end of every task in a repo that has a .whylog.toml, when asked why something was done a certain way, when asked to log/compact walkthroughs or sessions, or to set up a decision log for a project.
---

# whylog

`whylog` keeps per-run session walkthroughs and a compacted decision log (`**what** - *why*`)
so every agent and human on a repo can see what was decided and why. It is concurrency-safe:
many agents can open, note and close sessions at once.

## Install (if `whylog` is not on PATH)

```sh
curl -fsSL https://raw.githubusercontent.com/stuyk/whylog/main/install.sh | sh
# Windows: irm https://raw.githubusercontent.com/stuyk/whylog/main/install.ps1 | iex
# or: cargo install --git https://github.com/stuyk/whylog
```

## Every task

0. `whylog context` - the briefing for this repo: how to run the session, writing rules,
   recent and open sessions.
1. `whylog why <area terms>` - read what was already decided. If your plan contradicts a
   decision, ask the user before undoing it. `whylog recent` shows the last sessions.
2. `whylog open "<short topic>"` - note the 4-hex handle it prints (e.g. `9f3a`).
3. While working: `whylog note 9f3a "<finding and why>"`. For a lasting rule (a convention,
   removal, revert, "we do X not Y", gotcha): `whylog decide 9f3a "**what** - *why*"`.
4. Finish: `whylog close 9f3a --summary "<one line>" --files <changed files...>`.

Use the commands instead of editing walkthrough files; they are cheaper than read+edit and
never break the header. Pass `-` instead of text to read multi-line stdin.

## How to write

Everything you record follows `whylog style`. In short: direct, active voice, no hedging or
filler, short sentences, real numbers and names, **no em-dashes** (use a period, comma,
colon or parentheses), no profanity. Decisions are `**the rule** - *the real reason*`.

## Compacting (recommend it once 25+ sessions are waiting)

`whylog context` and `whylog close` tell you when compaction is due (`compact_after`,
default 25). Finish the user's task first, then recommend it.

1. `whylog compact > digest.txt` and read it (`S:` summary, `D:` recorded decisions, `N:` notes).
2. Write one bullet per lasting decision, optionally grouped: `[Area] - **what** - *why*`.
   Merge duplicates, keep the newest rule, list reverted experiments so they aren't retried.
3. `whylog compact --apply decisions.txt --through <ID from the digest header>`.

## Setting up a project

`whylog init` (or `whylog init --sessions docs/walkthroughs --decisions docs/decisions.md`),
which also writes the managed whylog block into AGENTS.md so every agent knows about it.
Refresh it with `whylog agents`; `whylog agents --check` fails if it's missing or stale.

## Must not

- Edit a walkthrough's `- Topic/Opened/Closed/Status` header lines by hand.
- Record one-off tuning values as decisions.
- Run `compact --apply` without reading the digest first.
