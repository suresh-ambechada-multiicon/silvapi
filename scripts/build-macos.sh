#!/usr/bin/env bash
# Best-effort macOS cross-build from Linux via cargo-zigbuild.
#
# ┌── READ THIS ────────────────────────────────────────────────────────────┐
# │ This is EXPERIMENTAL. GPUI compiles Metal shaders with Apple's `metal`   │
# │ compiler at build time, which cannot run on Linux. This build will very  │
# │ likely fail at the GPUI shader step. For a guaranteed working macOS      │
# │ build, use real Apple hardware (GitHub Actions macos-latest).            │
# └─────────────────────────────────────────────────────────────────────────┘
#
# You must supply a macOS SDK yourself (Apple's Xcode EULA restricts it to
# Apple hardware — obtaining/using it is your responsibility):
#
#   1. On a Mac with Xcode: cd $(xcrun --show-sdk-path)/../..  (…/SDKs)
#      tar -cJf MacOSX.sdk.tar.xz MacOSX.sdk
#   2. Copy it into this repo at:  .cache/macos-sdk/MacOSX.sdk.tar.xz
#      (or point MACOS_SDK_TARBALL at it)
#
# Target arch: set MACOS_TARGET (default aarch64-apple-darwin; also
# x86_64-apple-darwin).
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="${MACOS_TARGET:-aarch64-apple-darwin}"
target_root="$repo_dir/.cache/target-macos"
target_dir="$target_root/$target/release"
dist_dir="$repo_dir/dist"
sdk_dir="$repo_dir/.cache/macos-sdk"
sdk_tarball="${MACOS_SDK_TARBALL:-$sdk_dir/MacOSX.sdk.tar.xz}"

# shellcheck source=scripts/build-common.sh
source "$repo_dir/scripts/build-common.sh"

# --- SDK check / auto-download / extract ---------------------------------------
# If no SDK present, fetch one from the osxcross community mirror. Apple's Xcode
# EULA restricts the SDK to Apple hardware — using a mirror is a gray area and
# your responsibility. Set MACOS_SDK_NO_DOWNLOAD=1 to require a local SDK instead.
MACOS_SDK_VERSION="${MACOS_SDK_VERSION:-14.5}"
sdk_mirror_url="https://github.com/joseluisq/macosx-sdks/releases/download/${MACOS_SDK_VERSION}/MacOSX${MACOS_SDK_VERSION}.sdk.tar.xz"

mkdir -p "$sdk_dir" "$target_root" "$dist_dir"

# Resolve the extracted SDK dir (mirror tarballs unpack to MacOSX<ver>.sdk).
sdk_extracted="$sdk_dir/MacOSX.sdk"
[ -d "$sdk_extracted" ] || sdk_extracted="$sdk_dir/MacOSX${MACOS_SDK_VERSION}.sdk"

if [ ! -d "$sdk_extracted" ]; then
  # Re-download if the tarball is missing or corrupt (truncated download →
  # `xz -t` fails, and extraction silently drops usr/include).
  if [ -f "$sdk_tarball" ] && ! xz -t "$sdk_tarball" 2>/dev/null; then
    echo "Cached SDK tarball is corrupt (failed xz integrity check) — re-downloading."
    rm -f "$sdk_tarball"
  fi
  if [ ! -f "$sdk_tarball" ]; then
    if [ "${MACOS_SDK_NO_DOWNLOAD:-0}" = "1" ]; then
      cat >&2 <<EOF
ERROR: macOS SDK not found and MACOS_SDK_NO_DOWNLOAD=1.

Expected extracted SDK at $sdk_dir/MacOSX*.sdk or tarball at $sdk_tarball.
See the header of this script for how to produce MacOSX.sdk.tar.xz from Xcode.
EOF
      exit 1
    fi
    echo "macOS SDK not found — downloading MacOSX${MACOS_SDK_VERSION}.sdk from community mirror."
    echo "  (Apple EULA gray area — see script header. Set MACOS_SDK_NO_DOWNLOAD=1 to disable.)"
    echo "  $sdk_mirror_url"
    curl -fL --retry 5 -C - -o "$sdk_tarball" "$sdk_mirror_url"
    if ! xz -t "$sdk_tarball" 2>/dev/null; then
      echo "ERROR: downloaded SDK tarball failed integrity check." >&2
      rm -f "$sdk_tarball"
      exit 1
    fi
  fi
  echo "Extracting macOS SDK from $sdk_tarball ..."
  tar -xf "$sdk_tarball" -C "$sdk_dir"
  sdk_extracted="$sdk_dir/MacOSX.sdk"
  [ -d "$sdk_extracted" ] || sdk_extracted="$sdk_dir/MacOSX${MACOS_SDK_VERSION}.sdk"
fi

if [ ! -d "$sdk_extracted" ]; then
  echo "ERROR: SDK extraction did not yield an SDK dir under $sdk_dir" >&2
  exit 1
fi

# Sanity: a full SDK has C headers under usr/include. An empty one means a
# stripped or truncated SDK that will break bindgen (media/Security headers).
if [ -z "$(ls -A "$sdk_extracted/usr/include" 2>/dev/null)" ]; then
  echo "ERROR: $sdk_extracted/usr/include is empty — SDK is stripped/truncated." >&2
  echo "Delete $sdk_dir and re-run, or supply a full SDK via MACOS_SDK_TARBALL." >&2
  exit 1
fi

print_build_plan
echo "  target: $target"
echo "  sdk:    $sdk_extracted"

docker build -f "$repo_dir/Dockerfile.macos" -t silvapi-build:macos "$repo_dir"

# `:z` relabels bind mounts for SELinux (Fedora/RHEL); harmless elsewhere.
docker run --rm \
  "${common_docker_env[@]}" \
  "${common_docker_volumes[@]}" \
  -v "$repo_dir:/work/silvapi:z" \
  -v "$target_root:/work/silvapi/target:z" \
  -v "$sdk_extracted:/opt/MacOSX.sdk:z" \
  -v "$CACHE_CARGO_REGISTRY:/usr/local/cargo/registry:z" \
  -v "$CACHE_CARGO_GIT:/usr/local/cargo/git:z" \
  -e "SDKROOT=/opt/MacOSX.sdk" \
  -e "MACOS_TARGET=$target" \
  -w /work/silvapi \
  silvapi-build:macos \
  bash -lc 'set -euo pipefail
    command -v zig
    export LIBCLANG_PATH="$(llvm-config --libdir 2>/dev/null || echo /usr/lib/llvm-*/lib)"
    # bindgen (media crate, etc.) must target the mounted macOS SDK.
    export BINDGEN_EXTRA_CLANG_ARGS="-isysroot /opt/MacOSX.sdk -I /opt/MacOSX.sdk/usr/include -F /opt/MacOSX.sdk/System/Library/Frameworks"
    # Replace zed `media` build.rs with a cross-build-friendly version so it
    # generates bindings for the TARGET os (not the Linux host).
    patch_src="/work/silvapi/patches/media-build-crossbuild.rs"
    while IFS= read -r build_rs; do
      cp "$patch_src" "$build_rs"
      echo "patched media build.rs: $build_rs"
    done < <(find /usr/local/cargo/git/checkouts -path "*/crates/media/build.rs" -type f)
    /usr/local/cargo/bin/cargo fetch --target "$MACOS_TARGET"
    /usr/local/cargo/bin/cargo zigbuild --release --target "$MACOS_TARGET"
    sccache --show-stats || true'

bin="$target_dir/silvapi"
if [ -f "$bin" ]; then
  cp "$bin" "$dist_dir/silvapi-macos-${target%%-*}"
  ls -lh "$dist_dir/silvapi-macos-${target%%-*}"
else
  echo "Build did not produce $bin (see errors above)." >&2
  exit 1
fi
