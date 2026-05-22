# tmnl-protocol — Plan & Roadmap

Working roadmap. The shipped surface is documented in [`README.md`](../README.md);
the user-facing summary in [`CHANGELOG.md`](../CHANGELOG.md).

This crate is deliberately small. The goal is *stability*, not feature growth —
it should change rarely, and every change should be additive.

---

## Roadmap

- [ ] **Capability negotiation** — extend `Hello` so each side advertises a
      feature set, letting the protocol grow without breaking older peers
      (tracked from the `tmnl` side too).
- [ ] **An ergonomic client layer** — a thin `Client` helper (connect +
      handshake + a frame builder) so a backing app is ~20 lines, not ~100.
      Currently apps hand-roll the socket loop.
- [ ] **Richer input** — hover regions, focus enter/leave, IME / composition
      events.
- [ ] **Documented frame-diff guidance** — best practices for producing minimal
      `DiffRun` sets, so every client redraws efficiently.
- [ ] **1.0** — once the message set has settled and capability negotiation is
      in, freeze the format and commit to SemVer stability.

## Design notes

- **Additive only.** New message types append to the `Message` enum with a fresh
  type tag; older peers reject unknown tags cleanly. Never reshape an existing
  message — that's a silent breakage across every peer.
- **Defensive decoding is non-negotiable.** Every length/count off the wire is
  sanity-capped before use. Malformed input yields an `io::Error`, never a panic.
- **Zero dependencies.** The crate is pure `std`. Keep it that way — it's a
  path dependency of three other repos, and a heavy dep tree here is a tax on
  all of them.
- **The round-trip tests are the spec.** Every variant must encode → decode →
  compare-equal. If it isn't round-trip tested, it isn't really defined.
