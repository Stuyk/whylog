.PHONY: build test smoke release clean

build:
	cargo build --release

test:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test

# End-to-end: init, open, note, decide, close, why, compact, apply in a temp dir.
smoke:
	@cargo build --release -q && bin=$$(pwd)/target/release/whylog && dir=$$(mktemp -d) && cd "$$dir" && \
	$$bin init >/dev/null && h=$$($$bin open "smoke test" | awk '{print $$2}') && \
	$$bin note $$h "found a thing" >/dev/null && \
	$$bin decide $$h "**we do X** — *Y broke*" >/dev/null && \
	$$bin close $$h --summary "smoke" --files a.txt >/dev/null && \
	$$bin why "we do" | grep -q "we do X" && \
	thr=$$($$bin compact | sed -n 's/.*--through \([^ ]*\).*/\1/p') && \
	echo "- **we do X** — *Y broke*" | $$bin compact --apply - --through "$$thr" >/dev/null && \
	grep -q "## Compaction" DECISIONS.md && grep -q "whylog:compacted-through $$thr" DECISIONS.md && \
	test -z "$$(ls .whylog/sessions)" && echo "smoke ok" && cd / && rm -rf "$$dir"

# Cut a release from the CLI: make release V=0.2.0
# Bumps Cargo.toml, commits, creates tag vX.Y.Z and pushes both; the tag triggers
# .github/workflows/release.yml, which builds and publishes every platform.
release:
	@test -n "$(V)" || (echo "usage: make release V=X.Y.Z"; exit 1)
	@git diff --quiet && git diff --cached --quiet || (echo "commit or stash changes first"; exit 1)
	sed -i.bak 's/^version = ".*"/version = "$(V)"/' Cargo.toml && rm -f Cargo.toml.bak
	cargo check -q
	git commit -am "release: v$(V)"
	git tag -a "v$(V)" -m "whylog v$(V)"
	git push origin HEAD "v$(V)"

clean:
	cargo clean
	rm -rf dist
