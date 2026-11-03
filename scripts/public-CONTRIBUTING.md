# Contributing

LastHarness is developed in a private repository and copied here from time to time, so each commit in this
repository is a snapshot of development, not a single change.

- Build: `cargo build --workspace`. Test: `cargo test --workspace`. Rust 1.89 or newer.
- The page viewers are separate cargo workspaces: `crates/lh-pageview-engine` (Blitz) and `crates/lh-pageview-servo`
  (Servo, a large build). Build them with their `build.sh`.
- Issues and pull requests are welcome. A pull request is applied in the development repository and comes back
  here with the next snapshot.
- Licence: PolyForm Noncommercial 1.0.0 (see `LICENSE` and `COMMERCIAL-LICENSING.md`). The Servo helper is MPL-2.0, and `third_party/` keeps its own licences.
