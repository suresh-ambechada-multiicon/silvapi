#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_root="$repo_dir/.cache/target-windows"
target_dir="$target_root/x86_64-pc-windows-msvc/release"
dist_dir="$repo_dir/dist"
xwin_cache_dir="$repo_dir/.cache/xwin"

# shellcheck source=scripts/build-common.sh
source "$repo_dir/scripts/build-common.sh"

mkdir -p "$xwin_cache_dir" "$target_root" "$dist_dir"
print_build_plan

docker build -f "$repo_dir/Dockerfile.windows" -t silvapi-build:windows "$repo_dir"

# `:z` on every bind mount relabels it for SELinux (Fedora) — without it the
# container gets "Permission denied" (e.g. creating /root/.cache/cargo-xwin).
docker run --rm \
  "${common_docker_env[@]}" \
  "${common_docker_volumes[@]}" \
  -v "$repo_dir:/work/silvapi:z" \
  -v "$target_root:/work/silvapi/target:z" \
  -v "$xwin_cache_dir:/root/.cache:z" \
  -v "$CACHE_CARGO_REGISTRY:/usr/local/cargo/registry:z" \
  -v "$CACHE_CARGO_GIT:/usr/local/cargo/git:z" \
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
      if git -C "$checkout_dir" apply --reverse --check "$patch_file" >/dev/null 2>&1; then
        echo "GPUI Windows runtime shader patch already applied"
      elif git -C "$checkout_dir" apply --check "$patch_file" >/dev/null 2>&1; then
        git -C "$checkout_dir" apply "$patch_file"
      else
        echo "Failed to apply GPUI Windows runtime shader patch" >&2
        exit 1
      fi
    done < <(find /usr/local/cargo/git/checkouts -path "*/crates/gpui_windows/src/directx_renderer.rs" -type f)
    /usr/local/cargo/bin/cargo xwin build --release --target x86_64-pc-windows-msvc --xwin-http-retries 10
    sccache --show-stats || true'

cp "$target_dir/silvapi.exe" "$dist_dir/Silvapi.exe"
ls -lh "$dist_dir/Silvapi.exe"
