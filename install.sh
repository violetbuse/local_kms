#!/usr/bin/env bash
# Installs or updates local_kms from the latest GitHub release.
#
# Usage:
#   ./install.sh
#   curl -fsSL https://raw.githubusercontent.com/violetbuse/local_kms/master/install.sh | bash
#
# Env overrides:
#   LOCAL_KMS_REPO         GitHub "owner/repo" to install from (default: violetbuse/local_kms)
#   LOCAL_KMS_INSTALL_DIR  Directory to install the binary into (default: /usr/local/bin)
#
# This script never modifies shell config files. It installs into a directory that's
# normally already on PATH; if that directory isn't on PATH, it just prints a warning.

set -euo pipefail

REPO="${LOCAL_KMS_REPO:-violetbuse/local_kms}"
BIN_NAME="local_kms"
INSTALL_DIR="${LOCAL_KMS_INSTALL_DIR:-/usr/local/bin}"
VERSION_FILE="$INSTALL_DIR/.$BIN_NAME.version"

log() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
err() { printf 'error: %s\n' "$*" >&2; exit 1; }

command -v curl >/dev/null 2>&1 || err "curl is required but not found"
command -v tar >/dev/null 2>&1 || err "tar is required but not found"

# ---- detect platform ----

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
    Linux)
        case "$arch" in
            x86_64) target="x86_64-unknown-linux-gnu" ;;
            aarch64 | arm64) target="aarch64-unknown-linux-gnu" ;;
            *) err "unsupported Linux architecture: $arch" ;;
        esac
        ;;
    Darwin)
        case "$arch" in
            arm64) target="aarch64-apple-darwin" ;;
            x86_64) err "Intel Macs (x86_64-apple-darwin) are not built by $REPO's releases yet; build from source with 'cargo build --release' instead" ;;
            *) err "unsupported macOS architecture: $arch" ;;
        esac
        ;;
    *)
        err "unsupported OS: $os (only Linux and macOS are supported)"
        ;;
esac

asset="$BIN_NAME-$target.tar.gz"

# ---- resolve latest release tag (no jq/API-token dependency) ----

log "Checking latest release of $REPO..."
latest_url="$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")" \
    || err "could not reach GitHub to resolve the latest release"
latest_tag="${latest_url##*/}"
[ -n "$latest_tag" ] && [ "$latest_tag" != "latest" ] || err "could not determine the latest release tag"

installed_tag=""
if [ -f "$VERSION_FILE" ]; then
    installed_tag="$(cat "$VERSION_FILE")"
fi

if [ "$installed_tag" = "$latest_tag" ] && [ -x "$INSTALL_DIR/$BIN_NAME" ]; then
    log "$BIN_NAME $latest_tag is already installed and up to date ($INSTALL_DIR/$BIN_NAME)"
    exit 0
fi

if [ -n "$installed_tag" ]; then
    log "Updating $BIN_NAME $installed_tag -> $latest_tag"
else
    log "Installing $BIN_NAME $latest_tag"
fi

# ---- download + extract ----

download_url="https://github.com/$REPO/releases/download/$latest_tag/$asset"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

log "Downloading $asset..."
curl -fsSL "$download_url" -o "$tmp_dir/$asset" \
    || err "failed to download $download_url"

tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"

extracted_bin="$tmp_dir/$BIN_NAME-$target/$BIN_NAME"
[ -f "$extracted_bin" ] || err "downloaded archive did not contain the expected binary"

# ---- install (using sudo only if the install directory isn't writable) ----

SUDO=""
if [ -d "$INSTALL_DIR" ] && [ -w "$INSTALL_DIR" ]; then
    :
elif [ "$(id -u)" -eq 0 ]; then
    :
elif command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
    log "$INSTALL_DIR requires elevated privileges; you may be prompted for your password"
else
    err "install directory is not writable and sudo is not available: $INSTALL_DIR (set LOCAL_KMS_INSTALL_DIR to a writable directory instead)"
fi

$SUDO mkdir -p "$INSTALL_DIR" || err "could not create install directory: $INSTALL_DIR"
$SUDO install -m 755 "$extracted_bin" "$INSTALL_DIR/$BIN_NAME"
printf '%s\n' "$latest_tag" | $SUDO tee "$VERSION_FILE" >/dev/null

log "Installed $BIN_NAME $latest_tag to $INSTALL_DIR/$BIN_NAME"

# ---- PATH check (informational only; this script never edits shell configs) ----

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) warn "$INSTALL_DIR is not on your PATH. Add it yourself, e.g.: export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac

log "Done. Run '$BIN_NAME' to start the server."
