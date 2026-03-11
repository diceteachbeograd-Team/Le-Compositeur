# Test Matrix

Last updated: 2026-03-11

## Goal

Provide reproducible smoke checks for the supported Linux families and keep render-overlay regressions detectable.

## Standard Automated Smoke Suite

Run this from repo root on every target distro:

```bash
tmpdir="$(mktemp -d)"
cfg="${tmpdir}/config.toml"

cargo test --all --quiet
cargo test -q -p wc-render native_bmp_overlay_output_hash_is_stable -- --nocapture

cargo run -q -p wc-cli -- init --config "$cfg" --force
cargo run -q -p wc-cli -- validate --config "$cfg"
cargo run -q -p wc-cli -- render-preview --config "$cfg"
cargo run -q -p wc-cli -- run --config "$cfg" --once
```

Pass criteria:
- all commands exit `0`
- render regression hash test is stable
- `wc-cli render-preview` writes `current.png` and metadata for the temp config run

## Distro Matrix

### Fedora Workstation (GNOME, Wayland)

```bash
sudo dnf install -y rust cargo rpm-build rpmdevtools desktop-file-utils rsync
./scripts/build-alpha-rpm.sh 2026.03.11-1
```

Then execute the standard automated smoke suite.

### Fedora KDE Spin (Wayland)

```bash
sudo dnf install -y rust cargo rpm-build rpmdevtools desktop-file-utils rsync
./scripts/build-alpha-rpm.sh 2026.03.11-1
```

Then execute the standard automated smoke suite.

### Ubuntu LTS (GNOME, Wayland)

```bash
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.11-1
```

Then execute the standard automated smoke suite.

### Debian Stable

```bash
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.11-1
```

Then execute the standard automated smoke suite.

### openSUSE Tumbleweed

```bash
sudo zypper install -y rust cargo rpm-build rpmdevtools desktop-file-utils rsync
```

Then execute the standard automated smoke suite.

## Overlay Regression Workflow

The baseline snapshot/regression guard is:

```bash
cargo test -q -p wc-render native_bmp_overlay_output_hash_is_stable -- --nocapture
```

If this test fails after intentional renderer changes:
1. Verify visual output change is expected.
2. Re-run test to capture reported `left` hash value.
3. Update expected hash in `crates/wc-render/src/lib.rs` test `native_bmp_overlay_output_hash_is_stable`.
4. Re-run `cargo test --all --quiet`.
