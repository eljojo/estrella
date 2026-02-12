# Releasing estrella

This document covers the full release process for publishing `.deb` packages via a flat apt repository hosted on GitHub Releases.

## One-time setup

### 1. Generate a GPG signing key

This key signs the apt repository metadata so users can verify packages are authentic.

The keyring is stored in `.gnupg/` at the project root (gitignored). When you enter `nix develop`, `GNUPGHOME` is automatically set to this directory, so keys are portable across machines — just copy the `.gnupg/` folder.

```bash
# Enter the dev shell (sets GNUPGHOME=.gnupg/)
nix develop

# Generate a new key (RSA 4096, no passphrase for CI use)
gpg --full-generate-key
# Select: (1) RSA and RSA
# Key size: 4096
# Expiration: 0 (does not expire), or set a reasonable expiration
# Name: estrella release signing
# Email: your email
# Passphrase: leave empty (press Enter twice)

# Find your key ID
gpg --list-keys --keyid-format long
# Look for the line like: pub   rsa4096/ABCDEF1234567890
# The part after the slash is your KEY_ID
```

### 2. Add GitHub secrets

Export the private key and add it as a repository secret:

```bash
# Export private key (armor format)
gpg --armor --export-secret-keys YOUR_KEY_ID
```

Go to your GitHub repo → Settings → Secrets and variables → Actions, and add:

| Secret | Value |
|--------|-------|
| `GPG_PRIVATE_KEY` | The full armor-encoded private key output (including `-----BEGIN PGP PRIVATE KEY BLOCK-----` and `-----END PGP PRIVATE KEY BLOCK-----`) |
| `GPG_KEY_ID` | Your key ID (e.g., `ABCDEF1234567890`) |

That's it — no GitHub Pages setup needed. The `apt` release is created automatically by the first CI run.

## Releasing a new version

```bash
make bump-patch-release
```

This fetches tags from GitHub, bumps the patch version (e.g., `v0.1.0` → `v0.1.1`), updates `Cargo.toml`, commits, tags, and pushes. The CI takes over from there.

For manual releases or non-patch bumps:

1. Update the version in `Cargo.toml`
2. Commit: `git commit -am "release: v0.2.0"`
3. Tag and push: `git tag v0.2.0 && git push && git push --tags`

The CI workflow automatically:
- Builds a static arm64 binary (no native dependencies)
- Packages it as a `.deb` with systemd services
- Creates a GitHub Release with the `.deb` attached
- Updates the flat apt repository on the `apt` release

## How the apt repository works

The apt repo is hosted entirely on GitHub Releases — no gh-pages branch, no git history bloat. It uses two types of releases:

### Two releases per version

| Release | URL pattern | Contents |
|---------|------------|----------|
| **v0.1.1** (version tag) | `releases/download/v0.1.1/estrella_0.1.1_arm64.deb` | The `.deb` for this version + release notes. A normal release page for humans. |
| **apt** (permanent tag) | `releases/download/apt/Packages`, `releases/download/apt/estrella_*.deb`, etc. | ALL `.deb` files from every version, plus the apt metadata (`Packages`, `Release`, `InRelease`, `gpg.key`). This is what users point their `sources.list` at. |

The `apt` release is a long-lived "bucket" that accumulates every `.deb` ever published. Users configure their `sources.list` once and it never changes:

```
deb [arch=arm64 signed-by=...] https://github.com/eljojo/estrella/releases/download/apt ./
```

### Why a dedicated `apt` release?

The `Packages` file must list **all** available versions so users can install or pin any of them. If we put it on the version release (e.g., v0.1.1), it would only know about that one version. And `releases/latest/download/` only points to the most recent release, so old versions would disappear.

The dedicated `apt` release gives us a single stable URL that always has the complete package index.

### What's on the `apt` release

All files at `https://github.com/eljojo/estrella/releases/download/apt/`:

```
gpg.key                          # public GPG signing key
Packages                         # package index (lists all versions)
Packages.gz                      # compressed index
Release                          # repo metadata with checksums
Release.gpg                      # detached GPG signature
InRelease                        # inline-signed metadata
estrella_0.1.0_arm64.deb         # every release is kept
estrella_0.1.1_arm64.deb
estrella_0.2.0_arm64.deb
...
```

The `.deb` files are GitHub Release assets served from GitHub's CDN. apt follows the 302 redirects that GitHub uses for release downloads.

### How CI updates it

Each release:
1. Uploads the new `.deb` to the `apt` release
2. Downloads the existing `Packages` index from the `apt` release
3. Generates a new entry for the new `.deb` and appends it
4. Re-signs and re-uploads the metadata files (overwrites with `--clobber`)

## What the CI does

### Build job (runs on `ubuntu-24.04-arm`)

Native aarch64 build using Nix:
1. Installs Nix via `cachix/install-nix-action`
2. Runs `nix build .#static` — Nix handles Rust nightly, musl, Node.js, frontend build, everything
3. Packages the static binary with [nfpm](https://nfpm.goreleaser.com/) into a `.deb`
4. Creates a GitHub Release for the version tag

### Publish job (runs on `ubuntu-24.04`)

Updates the flat apt repository on the `apt` release:
1. Downloads the `.deb` from the build job
2. Uploads it to the `apt` release
3. Downloads all `.deb` files from the `apt` release
4. Regenerates package indices with `dpkg-scanpackages`
5. Signs the Release file with GPG
6. Uploads metadata to the `apt` release (overwriting old metadata)

## Verifying the apt repo

After a release, verify the repository is working:

```bash
# Check the package index
curl -sL https://github.com/eljojo/estrella/releases/download/apt/Packages

# Check the GPG key
curl -sL https://github.com/eljojo/estrella/releases/download/apt/gpg.key | gpg --show-keys

# Check the signed release
curl -sL https://github.com/eljojo/estrella/releases/download/apt/InRelease
```

## Testing a .deb locally

```bash
# On a Raspberry Pi (or any arm64 Debian system):
sudo dpkg -i estrella_0.1.0_arm64.deb
sudo apt-get install -f  # installs bluez dependency if needed

# Check installed files
dpkg -L estrella

# Test the binary
estrella --help
```

## Architecture

The `.deb` package contains:
- `/usr/bin/estrella` — statically linked binary (zero runtime deps)
- `/usr/lib/systemd/system/estrella.service` — HTTP daemon
- `/usr/lib/systemd/system/estrella-rfcomm.service` — Bluetooth RFCOMM setup
- `/etc/estrella/estrella.conf` — configuration (survives upgrades)

The binary is built with `--no-default-features` which disables HEIC/HEIF image support (requires libheif C library). Users should convert iPhone photos to JPEG before uploading. All other features work identically to the Nix build.
