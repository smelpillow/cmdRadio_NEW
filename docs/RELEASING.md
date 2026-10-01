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
git tag vX.Y.Z
git push origin Main --tags
```

Use the exact version from `Cargo.toml` for `X.Y.Z`. The tag must be new; pushing an existing tag does not create a new release run.

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

## Code signing (Windows, optional)

Unsigned binaries can be blocked by corporate EDR/antivirus (e.g. CrowdStrike Falcon) on machines where the file has low reputation/prevalence. If this becomes a recurring problem, options in order of effort:

1. **Azure Trusted Signing**: cheapest path to a publicly trusted signature, built for individual/small-project maintainers. Requires an Azure subscription and identity validation.
2. **Commercial code signing certificate** (DigiCert, Sectigo, SSL.com, etc.): broadest trust, recurring cost.
3. **Internal/self-signed certificate**: only trusted on machines where an organization's IT deploys the root certificate via policy; does not help the general public downloading releases from GitHub.

Once a `.pfx` certificate is available, sign the built binary with the Windows SDK's `signtool`:

```powershell
signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /f certificate.pfx /p "password" target\release\cmdradio.exe
signtool verify /pa target\release\cmdradio.exe
```

To automate this in `.github/workflows/release.yml`, add a signing step after `Build` and before `Package (Windows)` that pulls the certificate from a GitHub Actions secret (e.g. base64-encoded `.pfx` + password secrets), decodes it, and runs `signtool`. Do not add this step until the certificate/secrets actually exist, to avoid breaking the release pipeline.

