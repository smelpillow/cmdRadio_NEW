# Releasing

## Versioning

This project uses Semantic Versioning: `MAJOR.MINOR.PATCH`.

## Prepare a release

1. Update `Cargo.toml` version.
2. Update `CHANGELOG.md` under a new version section.
3. Commit all changes.

## Create release tag

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Automated GitHub Release

Pushing a `v*.*.*` tag triggers `.github/workflows/release.yml`:

- Builds binaries for:
  - `x86_64-pc-windows-msvc`
  - `x86_64-unknown-linux-gnu`
- Packages artifacts (`.zip` and `.tar.gz`)
- Publishes a GitHub Release with attached binaries

## First release checklist

- CI workflow green on `main`
- Manual smoke test done on Windows and Linux
- Changelog updated
- Tag pushed
- Release artifacts downloaded and tested
