#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workspace_dir="$(cd "$repo_dir/.." && pwd)"
target_dir="$repo_dir/target/x86_64-pc-windows-msvc/release"
dist_dir="$repo_dir/dist"
xwin_cache_dir="$repo_dir/.cache/xwin"
cargo_registry_dir="$repo_dir/.cache/cargo-registry"
cargo_git_dir="$repo_dir/.cache/cargo-git"

mkdir -p "$xwin_cache_dir" "$cargo_registry_dir" "$cargo_git_dir"
docker build -f "$repo_dir/Dockerfile.windows" -t silvapi-build:windows "$repo_dir"
docker run --rm \
  -v "$workspace_dir:/work" \
  -v "$repo_dir/target:/work/silvapi/target" \
  -v "$xwin_cache_dir:/root/.cache" \
  -v "$cargo_registry_dir:/usr/local/cargo/registry" \
  -v "$cargo_git_dir:/usr/local/cargo/git" \
  -e XWIN_HTTP_RETRIES=10 \
  -w /work/silvapi \
  silvapi-build:windows \
  bash -lc 'set -euo pipefail
    command -v clang-cl
    command -v lld-link
    command -v llvm-lib
    /usr/local/cargo/bin/cargo xwin cache xwin --xwin-http-retries 10
    /usr/local/cargo/bin/cargo fetch --target x86_64-pc-windows-msvc
    while IFS= read -r rc_file; do
      manifest_dir="$(dirname "$rc_file")"
      manifest_path="$manifest_dir/gpui.manifest.xml"
      if [ -f "$manifest_path" ]; then
        sed -i "s#\"resources/windows/gpui.manifest.xml\"#\"$manifest_path\"#" "$rc_file"
      fi
    done < <(find /usr/local/cargo/git/checkouts -path "*/crates/gpui/resources/windows/gpui.rc" -type f)
    while IFS= read -r renderer_file; do
      checkout_dir="${renderer_file%/crates/gpui_windows/src/directx_renderer.rs}"
      patch_file="/work/silvapi/patches/gpui-windows-runtime-shaders.patch"
      if git -C "$checkout_dir" apply --check "$patch_file"; then
        git -C "$checkout_dir" apply "$patch_file"
      elif git -C "$checkout_dir" apply --reverse --check "$patch_file"; then
        echo "GPUI Windows runtime shader patch already applied"
      else
        echo "Failed to apply GPUI Windows runtime shader patch" >&2
        exit 1
      fi
    done < <(find /usr/local/cargo/git/checkouts -path "*/crates/gpui_windows/src/directx_renderer.rs" -type f)
    /usr/local/cargo/bin/cargo xwin build --release --target x86_64-pc-windows-msvc --xwin-http-retries 10'

mkdir -p "$dist_dir"
cp "$target_dir/silvapi.exe" "$dist_dir/Silvapi.exe"
ls -lh "$dist_dir/Silvapi.exe"
