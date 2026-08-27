# Releasing

## Versioning

This project uses Semantic Versioning: `MAJOR.MINOR.PATCH`.

## Prepare a release

1. Update the version in `Cargo.toml` and the displayed application version in `src/app.rs`.
2. Update `CHANGELOG.md` under a new version section.
3. Update user-facing documentation when behavior changes.
4. Run the release checks:

```bash
cargo fmt --all -- --check
cargo check
cargo build --release
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

5. Run the release smoke test by launching the built binary once and validating that the app starts without immediate crashes.
6. Commit all release changes.

## Create release tag

```bash
git tag v0.4.10
git push origin v0.4.10
```

## Automated GitHub Release

Pushing a `v*.*.*` tag triggers `.github/workflows/release.yml`:

- Builds binaries for:
  - `x86_64-pc-windows-msvc`
  - `x86_64-unknown-linux-gnu`
- Packages artifacts (`.zip` and `.tar.gz`)
- Publishes a GitHub Release with attached binaries

The release workflow is tag-driven. A normal branch push or commit does not generate release artifacts. Before pushing the tag, verify that its version matches `Cargo.toml`, the application title, and the changelog; the workflow does not enforce this consistency automatically.

## First release checklist

- CI workflow green on `main`
- Version values are consistent across `Cargo.toml`, the application title, and `CHANGELOG.md`
- Manual smoke test done on Windows and Linux
- Production build succeeds: `cargo build --release`
- Format, check, test, and Clippy checks pass
- Changelog updated
- Tag pushed
- Release artifacts downloaded and tested
