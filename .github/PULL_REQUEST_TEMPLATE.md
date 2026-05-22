<!--
Thanks for contributing to tmnl-protocol! Before you open this PR:
  - Branch from `main`.
  - Run the verification gate: cargo fmt · build · clippy --all-targets · test.
  - Changing the wire format? Add, don't reshape — and round-trip test it.
See CONTRIBUTING.md for the full workflow.
-->

## Summary

<!-- What does this change, and which peers (tmnl / mnml / mixr) does it affect? -->

## How was it verified?

- [ ] `cargo fmt` clean
- [ ] `cargo clippy --all-targets` warning-free
- [ ] `cargo test` green
- [ ] Round-trip test added for any new / changed message

## Wire-format impact

<!-- None, additive (older peers ignore it), or breaking (PROTOCOL_VERSION bumped)? -->

## Related issues

<!-- e.g. Closes #123 -->
