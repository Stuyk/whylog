# whylog agent guide

whylog keeps two artifacts in your repo:

- **Session walkthroughs** (`sessions_dir`, one markdown file per agent run): what you did,
  why, and which files you touched. Concurrency-safe: many agents/humans can work at once.
- **A decision log** (`decisions_file`): lasting decisions, each written as
  `**what** - *why*`. Sessions are periodically compacted into it and pruned.

Read the decision log before you change established behaviour. If your change contradicts
an entry, ask the user instead of silently undoing a past decision.

## Every run

```sh
whylog context                     # 0. the briefing: rules, recent and open sessions
whylog why <terms>                 # 1. what was already decided about this area?
whylog recent                      #    what did the last sessions do?
whylog open "<short topic>"        # 2. prints a 4-hex HANDLE, e.g. `opened 9f3a (...) path`
whylog note 9f3a "found X because Y"            # 3. as you work (one line, or `-` for stdin)
whylog decide 9f3a "**we do X, not Y** - *Y broke Z*"   # lasting decisions only
whylog touch 9f3a src/a.rs src/b.rs             # files you changed (optional; close takes --files)
whylog close 9f3a --summary "one line" --files src/a.rs --decision "**…** - *…*"
```

- Use the handle, not the full id. Any unique part of the id works.
- Never hand-edit a walkthrough's header (`- Topic/Opened/Closed/Status`); use the commands.
- `note` and `decide` take text after the handle; pass `-` to read multi-line stdin.
- A decision is something a future agent must respect: a convention, a removal, a revert,
  a "we do X, not Y", or an engine/tool gotcha. Not one-off tuning values.

## How to write

Run `whylog style` for the full rules (plus any project rules from `style_file`). The short
version:

- Direct and in active voice. "I removed the retries", not "The retries were removed".
- No hedging, no apologizing, no filler ("Furthermore", "It should be noted", "leverage").
- Short sentences, short paragraphs. Real numbers and real names (files, functions, settings).
- **No em-dashes.** Use a period, comma, colon or parentheses. whylog converts any it gets.
- No profanity.
- Decisions: `- **the rule** - *the real reason*`, one per line.

## Compaction (recommend it once 25+ sessions are waiting)

whylog tracks how many closed sessions haven't been compacted. Once that hits
`compact_after` (default 25, set in `.whylog.toml`, 0 turns it off), `whylog context` and
`whylog close` say compaction is due. Finish the user's task, then recommend it.

```sh
whylog compact > /tmp/digest.txt        # token-lean digest of closed, uncompacted sessions
# read it; write one bullet per lasting decision to /tmp/decisions.txt:
#   [Area] - **what** - *why*      ([Area] optional; groups lines under ### Area)
whylog compact --apply /tmp/decisions.txt --through <ID printed in the digest>
```

`--apply` appends a dated `## Compaction YYYY-MM-DD (N sessions)` section to the decision
log, records a `whylog:compacted-through` marker, and deletes the compacted walkthroughs
(`--keep` to keep them). Sessions closed after the digest was made are never touched.

Prefer the session's own `D:` lines (recorded with `decide`/`--decision`), then infer from
`S:` (summary) and `N:` (notes). Merge duplicates, keep the newest rule when they conflict,
and mention reverted experiments explicitly so nobody retries them.

## Setup

```sh
whylog init                                        # .whylog.toml, sessions dir, DECISIONS.md
whylog init --sessions docs/walkthroughs --decisions docs/decisions.md
```

`init` also writes a managed whylog block into AGENTS.md (and CLAUDE.md unless it already
points to AGENTS.md). Refresh it with `whylog agents`; check it in CI with
`whylog agents --check`; remove it with `whylog agents --remove`.
