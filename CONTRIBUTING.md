# Contributing

Snapious is a hobby project. Bug reports, ideas and pull requests are welcome.

## Bugs and requests
Open an issue with the matching template. For bugs, please include your macOS version, steps to reproduce and, if possible, a small sample file (no private data).

## Code
1. Fork the repo and branch from `main`.
2. Keep changes small and explain *why* they are needed.
3. Before opening a pull request, these must pass:
   ```bash
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   scripts/bundle-app.sh && app/.build/debug/SnapiousSelfTest
   ```
4. New computation belongs in `crates/snapious-core` and gets tests with hand-checked expected values.
   The SwiftUI layer stays thin and calls the core through `crates/snapious-ffi`.
5. Coordinates in the core are 0-based with exclusive ends.

## Style
User-facing strings are written in English in the code (wrap them in `tr("…")`); the German translations live in `app/Sources/SnapiousApp/Translations.swift`. Keep the standard macOS look; do not invent a custom design system.
