# Contributing to tpt-chora

Thank you for your interest in contributing to tpt-chora. This guide
covers the development workflow, coding conventions, and the
verification steps required before a change can be merged.

## Development workflow

1. Fork the repository and create a branch for your change.
2. Make your changes.
3. Run the verification suite (see below).
4. Commit your changes with a clear, concise commit message.
5. Open a pull request against `main`.

## Coding conventions

- **No comments in code** unless the *why* is genuinely non-obvious
  (a hidden constraint, a workaround, a subtle invariant).
- **`cargo fmt`** must pass before committing. Run `cargo fmt --all`.
- **`cargo clippy --workspace --all-targets -- -D warnings`** must pass
  with zero warnings.
- **`cargo build --workspace --all-targets`** must pass.
- **`cargo test --workspace --all-targets`** must pass.
- **`cargo deny check`** must pass (supply-chain hygiene).
- Crate names follow the `tpt-chora-<name>` convention and live at
  `crates/tpt-chora-<name>`.
- The `tpt-chora` prefix is used for both the directory name and the
  package name.

## Verification

Run the full verification suite locally before opening a PR:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --all-targets
cargo test --workspace --all-targets
cargo deny check
```

On Windows, building `tpt-chora-render` requires a working MSVC linker
(Visual Studio Build Tools, "Desktop development with C++" workload).
The bundled GNU/mingw Rust toolchain's linker is link-only and cannot
build wgpu's Windows bindings.

## Platform-specific features

Three crates have opt-in, default-off Cargo features for platform-native
backends. These are feature-gated so they only compile when explicitly
enabled:

| Crate | Feature | Platforms |
| --- | --- | --- |
| `tpt-chora-a11y` | `native-a11y-backends` | Windows (verified), macOS (unverified), Android (unverified) |
| `tpt-chora-input` | `native-haptics-backends` | macOS (unverified), Android (unverified) |
| `tpt-chora-media` | `native-video-backends` | Linux VA-API (CI-verified), macOS (unverified), Android (unverified) |

## Architecture overview

See `ARCHITECTURE.md` for a crate-by-crate account of what's
implemented versus still directional. See `spec.txt` for the full
design document and `todo.md` for the phased roadmap.