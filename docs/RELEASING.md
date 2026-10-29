# Releasing Keyrust

Releases are built by [`.github/workflows/release.yml`](../.github/workflows/release.yml). A tag
`vX.Y.Z` on a commit of `main` builds the installers on GitHub's runners, signs their checksums
with [cosign](https://docs.sigstore.dev/cosign/) and creates a **draft** GitHub Release. Nothing
is public until a maintainer publishes the draft.

## What a release contains

| File | Platform |
|---|---|
| `Keyrust_X.Y.Z_x64-setup.exe` | Windows installer (NSIS) |
| `Keyrust_X.Y.Z_x64_en-US.msi` | Windows installer (MSI, for managed deployment) |
| `Keyrust_X.Y.Z_amd64.deb` | Debian, Ubuntu and derivatives |
| `Keyrust_X.Y.Z_amd64.AppImage` | Other Linux distributions |
| `Keyrust_X.Y.Z_universal.dmg` | macOS, Intel and Apple Silicon |
| `SHA256SUMS` | SHA-256 of the five installers |
| `SHA256SUMS.sigstore.json` | cosign signature bundle of `SHA256SUMS` |

How users verify them is in [SECURITY.md](../SECURITY.md#verifying-releases).

## Making a release

1. On a branch, set the new version in `Cargo.toml` (`[workspace.package] version`) and in
   `crates/app/ui/package.json`, update the `keyrust-*` entries of `Cargo.lock` and
   `fuzz/Cargo.lock` (`cargo metadata --offline` in both workspaces does it without touching other
   dependencies), and move the `[Unreleased]` notes of `CHANGELOG.md` under a `## [X.Y.Z]`
   heading. `scripts/release/check-version.sh vX.Y.Z` must print the version.
2. Open a pull request, wait for CI, merge it.
3. Tag the merge commit on `main` and push the tag:

   ```sh
   git switch main && git pull
   git tag vX.Y.Z          # or `git tag -s vX.Y.Z` to sign the tag with your key
   git push origin vX.Y.Z
   ```

4. Watch the **Release** workflow. When it is green, open the draft on the Releases page, read the
   notes (the CHANGELOG section plus a *Build* section: code-signing status and the reproducibility
   check) and download one installer to verify it as described in SECURITY.md.
5. Publish the draft.

### If the workflow fails

- **verify** fails when the tag does not match the version, the tagged commit is not on `main`, or
  only part of the code-signing secrets of an OS exists. Fix the cause; to move a tag that was
  never published: `git push --delete origin vX.Y.Z && git tag -d vX.Y.Z`, then tag again.
- A **build**, **checksums and signature** or **draft release** failure can be rerun from the
  Actions page (*Re-run failed jobs*). Rerunning a release whose draft already exists replaces its
  assets and notes; a published release is never modified, the job fails instead.
- A wrong draft can be deleted from the Releases page; the tag stays and can be reused.

### Dry runs

Every run that is not a pushed tag is a dry run: same builds, same checksums, same cosign
signature, no release and no OS code signing. The result is the `keyrust-release-dryrun` artifact
of the run (kept 7 days). Dry runs start:

- on pull requests to `main` that change the release workflow, `scripts/release/` or the Tauri
  configuration. Pull requests from forks and Dependabot get no OIDC token from GitHub, so their
  dry run skips the cosign signature;
- by hand from the Actions page (*Release* → *Run workflow*), on any branch or tag.

## Code signing (optional)

Without code signing, Windows SmartScreen and macOS Gatekeeper warn before the first launch, and
some corporate machines refuse unsigned installers entirely. The workflow signs automatically as
soon as the secrets of an OS exist; without them it still produces every installer, unsigned
(macOS builds get an ad-hoc signature, without which the app does not start on Apple Silicon).
Secrets are read only for a pushed tag, never for pull requests or manual runs.

All the secrets and variables of an OS must be set: a partial set makes the **verify** job fail
rather than silently produce unsigned installers.

Prices and eligibility change: check them when you sign up.

### Windows

| Option | Cost | Status in the workflow |
|---|---|---|
| [Azure Trusted Signing](https://learn.microsoft.com/azure/trusted-signing/) | Monthly subscription (about 10 USD/month at the time of writing); identity validation; eligibility for individuals depends on the country | **Wired**: set the secrets below |
| [SignPath Foundation](https://signpath.org/) | Free for open-source projects, after an application | Not wired: SignPath signs files uploaded to its service, so the workflow needs a signing step of its own |
| OV/EV certificate from a certificate authority | Yearly fee; the key lives on a hardware token or a cloud HSM | Not wired: a hardware token cannot be used from GitHub's runners |

Azure Trusted Signing uses [`trusted-signing-cli`](https://crates.io/crates/trusted-signing-cli)
through Tauri's `bundle.windows.signCommand`, which signs the app binary and both installers.

| Name | Kind | Value |
|---|---|---|
| `AZURE_CLIENT_ID` | secret | Client ID of the app registration allowed to sign |
| `AZURE_CLIENT_SECRET` | secret | Its client secret |
| `AZURE_TENANT_ID` | secret | The Azure tenant |
| `AZURE_SIGNING_ENDPOINT` | variable | The account's endpoint, e.g. `https://weu.codesigning.azure.net` |
| `AZURE_SIGNING_ACCOUNT` | variable | The Trusted Signing account name |
| `AZURE_SIGNING_PROFILE` | variable | The certificate profile name |

Secrets and variables are set under *Settings → Secrets and variables → Actions*.

Even when signed, a new certificate has no SmartScreen reputation yet, and a corporate policy may
still block software that is not on its allow list.

### macOS

Signing and notarization need the [Apple Developer Program](https://developer.apple.com/programs/)
(99 USD/year at the time of writing). Tauri signs with the *Developer ID Application* certificate
and notarizes with an App Store Connect API key.

| Name | Kind | Value |
|---|---|---|
| `APPLE_CERTIFICATE` | secret | The *Developer ID Application* certificate and key, exported as `.p12`, base64-encoded |
| `APPLE_CERTIFICATE_PASSWORD` | secret | The password of the `.p12` |
| `APPLE_SIGNING_IDENTITY` | secret | The identity name, e.g. `Developer ID Application: Name (TEAMID)` |
| `APPLE_API_ISSUER` | secret | Issuer ID of the App Store Connect API key |
| `APPLE_API_KEY` | secret | Key ID of that API key |
| `APPLE_API_PRIVATE_KEY` | secret | The `.p8` private key, base64-encoded |

> **Not yet exercised.** No certificate has been configured so far, so the signed paths of the
> workflow have not run. The first signed release should be checked by hand: `signtool verify /pa`
> on Windows, `spctl -a -vv` and `stapler validate` on macOS.

## Reproducing a build

Releases are reproducible on a best-effort basis: the inputs are pinned and the build is
normalised, and every release run rebuilds the Linux binary twice to check it. The installers
themselves (`.deb`, AppImage, `.dmg`, `.msi`, NSIS) are **not** claimed to be bit-for-bit
reproducible: they contain timestamps and tool output of their own.

What is pinned or normalised:

- Rust toolchain (`rust-toolchain.toml`), Node (`crates/app/ui/.node-version`) and pnpm
  (`packageManager` in `crates/app/ui/package.json`); dependencies from `Cargo.lock` (`--locked`)
  and `pnpm-lock.yaml` (`--frozen-lockfile`); GitHub Actions pinned to commit SHAs.
- `SOURCE_DATE_EPOCH` set to the commit time; SvelteKit uses it as its version name, which would
  otherwise be the build time embedded in the binary (`crates/app/ui/svelte.config.js`).
- Absolute paths remapped by `scripts/release/rustflags.sh` (checkout → `/build`, Cargo home →
  `/cargo`, rustup home → `/rustup`).
- No build cache.

Not pinned: the runner images (`ubuntu-24.04` changes with updates, `windows-latest` and
`macos-latest` move between versions) and the system libraries they provide.

The **reproducibility** job compares the two rebuilds; its result (`match`, `mismatch` or
`not available`) is in the run summary and in the release notes. It never blocks a release.

To rebuild the Linux binary of a release yourself, on Ubuntu 24.04 (or in an `ubuntu:24.04`
container) with the [Tauri prerequisites](DEVELOPMENT.md#linux-debianubuntu), rustup, Node and
pnpm at the pinned versions:

```sh
git clone https://github.com/downlevel/keyrust && cd keyrust
git checkout vX.Y.Z
export SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
export RUSTFLAGS="$(scripts/release/rustflags.sh)"
cd crates/app/ui
pnpm install --frozen-lockfile
pnpm tauri build --target x86_64-unknown-linux-gnu --no-bundle -- --locked
sha256sum ../../../target/x86_64-unknown-linux-gnu/release/keyrust
```

Compare the hash with the *Reproducibility check* line of the release notes. The binary inside the
`.deb` (`dpkg-deb -x Keyrust_X.Y.Z_amd64.deb out && sha256sum out/usr/bin/keyrust`) can differ from
it, because the Tauri bundler may patch the binary while packaging it; the run summary of the
**reproducibility** job shows both hashes.
