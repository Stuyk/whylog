# AGENTS.md

Guidance for AI coding agents working **on** this repo. To **use** whylog in another
project, see [`skills/whylog/SKILL.md`](skills/whylog/SKILL.md) or run `whylog guide`.

## What this is

whylog is a single dependency-free Rust binary that manages per-run session walkthroughs
and a compacted decision log (`**what** - *why*`) so agents can read why things were decided.

## Layout

| Path | Purpose |
|---|---|
| `src/main.rs` | CLI: argument parsing (no framework) and every command |
| `src/store.rs` | Walkthrough format: parse, section edits, id resolution, lock, atomic write |
| `src/compact.rs` | Digest, decision-line parsing, dated entries, compaction marker |
| `src/config.rs` | `.whylog.toml` discovery and parsing |
| `src/timefmt.rs` | UTC formatting without a date crate |
| `src/json.rs` | `--json` output without serde |
| `assets/GUIDE.md` | Printed by `whylog guide` (embedded) |
| `skills/whylog/SKILL.md` | Claude Code skill; embedded, printed/installed by `whylog skill` |
| `.github/workflows/` | `ci.yml` (fmt, clippy, test, smoke on 3 OSes), `release.yml` (on `v*` tags) |

## Commands

```sh
make test     # cargo fmt --check, clippy -D warnings, cargo test
make smoke    # end-to-end init → open → note → decide → close → why → compact → apply
make build    # target/release/whylog
```

Run `make test` and `make smoke` before finishing any change.

## Rules

- **Zero dependencies.** Keep `[dependencies]` empty; it keeps cross-compiling trivial and the binary small.
- **The walkthrough format is a contract.** Header lines (`- Topic/Opened/Closed/Status`) and section
  headings are parsed by other tools. Change them only with a migration path, and keep old files readable.
- Command output is terse on purpose (agents pay per token). Errors go to stderr, `--json` goes to stdout and must stay valid JSON.
- Every read-modify-write of a session or the decision log happens under `store::lock` and goes through `write_atomic`.
- Add a unit test for every behaviour change in `store`, `compact` or `config`.
- Changes to commands or flags must be reflected in `USAGE` (main.rs), `assets/GUIDE.md`, `skills/whylog/SKILL.md`, `README.md` and `llms.txt`.
- **Never create tags or releases by hand** unless asked. Releases are `make release V=X.Y.Z` (patch = fixes, minor = features, major = breaking format/CLI changes).
