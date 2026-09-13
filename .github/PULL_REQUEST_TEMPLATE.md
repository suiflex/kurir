## Summary

<!-- Why this change exists. One to three bullets. The diff already shows what changed. -->

-

## Changes

<!-- Actual edits, grouped by area. Mention affected harnesses and package channels. -->

-

## Test plan

<!-- Tick only checks that actually passed. -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --all-targets`
- [ ] `cargo test`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `npm test --prefix npm`
- [ ] Exercised registration through a real harness, if behavior changed

## Checklist

- [ ] Existing configuration entries remain preserved
- [ ] Dry-run does not write files or invoke delegated CLIs
- [ ] Secrets are redacted from previews and errors
- [ ] `CLAUDE.md` updated if architecture, invariants, or harness support changed
- [ ] No SuiFlex product migration is bundled into this change
