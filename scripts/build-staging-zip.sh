#!/usr/bin/env bash
# Build OpenHuman staging .app bundles for both Apple Silicon (arm64) and
# Intel (x86_64), then package each into a shareable zip.
#
# Signing is optional — if scripts/ci-secrets.json exists and contains
# APPLE_CERTIFICATE_BASE64 / APPLE_SIGNING_IDENTITY the .app is codesigned;
# otherwise it is built unsigned (Gatekeeper will block on first open;
# right-click → Open or: xattr -dr com.apple.quarantine OpenHuman.app).
#
# Usage:
#   bash scripts/build-staging-zip.sh                # both arches (default)
#   bash scripts/build-staging-zip.sh --arch arm64   # Apple Silicon only
#   bash scripts/build-staging-zip.sh --arch x86_64  # Intel only
#   bash scripts/build-staging-zip.sh --debug        # debug builds (faster)
#   bash scripts/build-staging-zip.sh --out ./my-dir # custom output directory
#
# Output (default: dist/staging/):
#   OpenHuman-staging-arm64-<git-sha>.zip    (M1/M2/M3/M4 Mac)
#   OpenHuman-staging-x86_64-<git-sha>.zip  (Intel Mac)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SECRETS_FILE="$ROOT_DIR/scripts/ci-secrets.json"

# ── Argument parsing ──────────────────────────────────────────────────────────
BUILD_ARCH="both"
BUILD_MODE="release"
OUT_DIR="$ROOT_DIR/dist/staging"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --arch)  BUILD_ARCH="${2:?--arch requires: arm64 | x86_64 | both}"; shift 2 ;;
    --debug) BUILD_MODE="debug"; shift ;;
    --out)   OUT_DIR="${2:?--out requires a path}"; shift 2 ;;
    -h|--help)
      sed -n '2,/^set/p' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

case "$BUILD_ARCH" in
  arm64)  TARGETS=("aarch64-apple-darwin") ;;
  x86_64) TARGETS=("x86_64-apple-darwin") ;;
  both)   TARGETS=("aarch64-apple-darwin" "x86_64-apple-darwin") ;;
  *) echo "Unknown --arch: $BUILD_ARCH (arm64 | x86_64 | both)" >&2; exit 1 ;;
esac

# ── Prerequisite checks ───────────────────────────────────────────────────────
for cmd in rustup ditto; do
  command -v "$cmd" &>/dev/null || { echo "[build-staging-zip] ERROR: '$cmd' not found" >&2; exit 1; }
done

GIT_SHA="$(git -C "$ROOT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")"
mkdir -p "$OUT_DIR"

echo "[build-staging-zip] mode=$BUILD_MODE arches=${TARGETS[*]} sha=$GIT_SHA out=$OUT_DIR"

# ── Staging environment ───────────────────────────────────────────────────────
export OPENHUMAN_APP_ENV=staging
export VITE_OPENHUMAN_APP_ENV=staging
export BACKEND_URL=https://staging-api.tinyhumans.ai
export VITE_BACKEND_URL=https://staging-api.tinyhumans.ai
export CEF_PATH="${CEF_PATH:-$HOME/Library/Caches/tauri-cef}"

source "$SCRIPT_DIR/load-dotenv.sh"

# ── Optional codesigning setup ────────────────────────────────────────────────
SIGNING_ENABLED=false

if [[ -f "$SECRETS_FILE" ]] && command -v jq &>/dev/null; then
  _CERT="$(jq -r '.secrets.APPLE_CERTIFICATE_BASE64 // ""' "$SECRETS_FILE")"
  _IDENT="$(jq -r '.secrets.APPLE_SIGNING_IDENTITY // ""' "$SECRETS_FILE")"
  if [[ -n "$_CERT" && -n "$_IDENT" ]]; then
    SIGNING_ENABLED=true
    source "$SCRIPT_DIR/load-env-json.sh" "$SECRETS_FILE" '.secrets + .vars'
  fi
fi

if $SIGNING_ENABLED; then
  echo "[build-staging-zip] codesigning ENABLED (identity: $APPLE_SIGNING_IDENTITY)"

  KEYCHAIN_NAME="build-staging.keychain-db"
  KEYCHAIN_PATH="$HOME/Library/Keychains/$KEYCHAIN_NAME"
  KEYCHAIN_PASSWORD="build-staging-$(date +%s)"

  cleanup_keychain() { security delete-keychain "$KEYCHAIN_PATH" 2>/dev/null || true; }
  trap cleanup_keychain EXIT
  cleanup_keychain

  security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
  security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
  security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"

  CERT_TMP=$(mktemp /tmp/build-staging-cert.XXXXXX.p12)
  trap 'cleanup_keychain; rm -f "$CERT_TMP"' EXIT
  echo "$APPLE_CERTIFICATE_BASE64" | base64 --decode > "$CERT_TMP"
  security import "$CERT_TMP" \
    -k "$KEYCHAIN_PATH" \
    -P "$APPLE_CERTIFICATE_PASSWORD" \
    -T /usr/bin/codesign -T /usr/bin/security
  rm -f "$CERT_TMP"
  security set-key-partition-list -S "apple-tool:,apple:,codesign:" \
    -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH" >/dev/null 2>&1
  EXISTING_KEYCHAINS=$(security list-keychains -d user | tr -d '"' | tr '\n' ' ')
  security list-keychains -d user -s "$KEYCHAIN_PATH" $EXISTING_KEYCHAINS
else
  echo "[build-staging-zip] codesigning SKIPPED (no credentials) — builds will be unsigned"
  # Ensure Tauri doesn't try to sign
  unset APPLE_SIGNING_IDENTITY 2>/dev/null || true
  unset APPLE_CERTIFICATE_BASE64 2>/dev/null || true
fi

# ── Chromium safe storage + tauri-cli ────────────────────────────────────────
bash "$SCRIPT_DIR/setup-chromium-safe-storage.sh"

cd "$ROOT_DIR/app"
pnpm tauri:ensure

# ── Build frontend once ───────────────────────────────────────────────────────
echo "[build-staging-zip] building frontend..."
pnpm run build

# ── Pre-add Rust targets before forking ──────────────────────────────────────
for RUST_TARGET in "${TARGETS[@]}"; do
  rustup target add "$RUST_TARGET"
done

# ── Launch both cargo builds in parallel ─────────────────────────────────────
LOG_DIR="$OUT_DIR/logs"
mkdir -p "$LOG_DIR"

BUILD_PIDS=()
BUILD_LOGS=()
BUILD_LABELS=()

for RUST_TARGET in "${TARGETS[@]}"; do
  case "$RUST_TARGET" in
    aarch64-apple-darwin) ARCH_LABEL="arm64" ;;
    x86_64-apple-darwin)  ARCH_LABEL="x86_64" ;;
  esac

  LOG_FILE="$LOG_DIR/build-${ARCH_LABEL}.log"
  echo "[build-staging-zip] launching $ARCH_LABEL build in background → $LOG_FILE"

  (
    # Skip beforeBuildCommand — we already built the frontend above.
    # Without this, parallel builds both run `pnpm run build:app` and race
    # on the shared app/dist/ directory, corrupting each other's output.
    BUILD_ARGS=(--bundles app,dmg --target "$RUST_TARGET" --config '{"build":{"beforeBuildCommand":""}}' -- --bin OpenHuman)
    [[ "$BUILD_MODE" == "debug" ]] && BUILD_ARGS=(--debug "${BUILD_ARGS[@]}")
    cargo tauri build "${BUILD_ARGS[@]}"
  ) >"$LOG_FILE" 2>&1 &

  BUILD_PIDS+=($!)
  BUILD_LOGS+=("$LOG_FILE")
  BUILD_LABELS+=("$ARCH_LABEL")
done

# ── Wait for both builds ──────────────────────────────────────────────────────
BUILD_OK=true
for i in "${!BUILD_PIDS[@]}"; do
  PID="${BUILD_PIDS[$i]}"
  ARCH_LABEL="${BUILD_LABELS[$i]}"
  LOG_FILE="${BUILD_LOGS[$i]}"
  echo "[build-staging-zip] waiting for $ARCH_LABEL build (pid $PID)..."
  if wait "$PID"; then
    echo "[build-staging-zip] $ARCH_LABEL build succeeded"
  else
    echo "[build-staging-zip] ERROR: $ARCH_LABEL build FAILED — last 40 lines:" >&2
    tail -40 "$LOG_FILE" >&2
    BUILD_OK=false
  fi
done
$BUILD_OK || exit 1

# ── Sign (if enabled) and zip each .app ──────────────────────────────────────
ENTITLEMENTS="$ROOT_DIR/app/src-tauri/entitlements.sidecar.plist"
CREATED_ZIPS=()

for RUST_TARGET in "${TARGETS[@]}"; do
  case "$RUST_TARGET" in
    aarch64-apple-darwin) ARCH_LABEL="arm64" ;;
    x86_64-apple-darwin)  ARCH_LABEL="x86_64" ;;
  esac

  echo
  echo "[build-staging-zip] === packaging $ARCH_LABEL ==="

  if [[ "$BUILD_MODE" == "debug" ]]; then
    BUNDLE_DIR="$ROOT_DIR/app/src-tauri/target/$RUST_TARGET/debug/bundle"
  else
    BUNDLE_DIR="$ROOT_DIR/app/src-tauri/target/$RUST_TARGET/release/bundle"
  fi

  APP_PATH="$(find "$BUNDLE_DIR/macos" -maxdepth 1 -name '*.app' 2>/dev/null | head -1)"
  if [[ -z "$APP_PATH" ]]; then
    echo "[build-staging-zip] ERROR: no .app found in $BUNDLE_DIR/macos" >&2
    exit 1
  fi
  echo "[build-staging-zip] .app: $APP_PATH"

  if $SIGNING_ENABLED; then
    MAIN_EXE="$(defaults read "$APP_PATH/Contents/Info.plist" CFBundleExecutable 2>/dev/null || echo "OpenHuman")"
    for bin in "$APP_PATH/Contents/MacOS/"*; do
      [[ -f "$bin" && -x "$bin" ]] || continue
      [[ "$(basename "$bin")" == "$MAIN_EXE" ]] && continue
      echo "  re-signing sidecar: $(basename "$bin")"
      codesign --force --options runtime \
        --entitlements "$ENTITLEMENTS" --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$bin"
    done
    echo "  re-signing .app bundle..."
    codesign --force --options runtime \
      --entitlements "$ENTITLEMENTS" --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$APP_PATH"
    codesign --verify --deep --strict "$APP_PATH"
  fi

  # Prefer the DMG if Tauri produced one; fall back to zipping the .app.
  DMG_PATH="$(find "$BUNDLE_DIR/dmg" -maxdepth 1 -name '*.dmg' 2>/dev/null | head -1)"
  if [[ -n "$DMG_PATH" ]]; then
    ZIP_NAME="OpenHuman-staging-${ARCH_LABEL}-${GIT_SHA}.dmg.zip"
    ZIP_PATH="$OUT_DIR/$ZIP_NAME"
    echo "[build-staging-zip] zipping DMG → $ZIP_PATH"
    ditto -c -k --keepParent "$DMG_PATH" "$ZIP_PATH"
  else
    ZIP_NAME="OpenHuman-staging-${ARCH_LABEL}-${GIT_SHA}.zip"
    ZIP_PATH="$OUT_DIR/$ZIP_NAME"
    echo "[build-staging-zip] no DMG found — zipping .app → $ZIP_PATH"
    ditto -c -k --keepParent "$APP_PATH" "$ZIP_PATH"
  fi

  CREATED_ZIPS+=("$ZIP_PATH")
  echo "[build-staging-zip] done: $ZIP_PATH ($(du -sh "$ZIP_PATH" | cut -f1))"
done

# ── Summary ───────────────────────────────────────────────────────────────────
echo
echo "===== build-staging-zip complete ====="
echo "  Git SHA:  $GIT_SHA"
echo "  Mode:     $BUILD_MODE"
echo "  Signed:   $SIGNING_ENABLED"
echo
for ZIP_PATH in "${CREATED_ZIPS[@]}"; do
  echo "  $(basename "$ZIP_PATH")  ($(du -sh "$ZIP_PATH" | cut -f1))"
  echo "  → $ZIP_PATH"
  echo
done
if ! $SIGNING_ENABLED; then
  echo "NOTE: builds are unsigned. To open on macOS:"
  echo "  xattr -dr com.apple.quarantine OpenHuman.app"
  echo "  (or right-click the .app → Open)"
fi
