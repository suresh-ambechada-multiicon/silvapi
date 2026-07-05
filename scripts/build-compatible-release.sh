#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="$repo_dir/dist"
target_dir="$repo_dir/.cache/target-linux"

# shellcheck source=scripts/build-common.sh
source "$repo_dir/scripts/build-common.sh"

mkdir -p "$dist_dir" "$target_dir"
print_build_plan

# Dockerfile.compat has no COPY steps (sources are bind-mounted at run time),
# so the repo itself is a small, sufficient build context.
docker build -f "$repo_dir/Dockerfile.compat" -t silvapi-build:bullseye "$repo_dir"

# `:z` on every bind mount relabels it for SELinux (Fedora) — without it the
# container can't read the repo ("Permission denied" / "could not find
# Cargo.toml") or write the caches. Mount only the repo (not the parent dir).
docker run --rm \
  "${common_docker_env[@]}" \
  "${common_docker_volumes[@]}" \
  -v "$CACHE_CARGO_REGISTRY:/cargo/registry:z" \
  -v "$CACHE_CARGO_GIT:/cargo/git:z" \
  -v "$repo_dir:/work/silvapi:z" \
  -v "$target_dir:/target:z" \
  -e CARGO_TARGET_DIR=/target \
  -w /work/silvapi \
  silvapi-build:bullseye \
  bash -c "set -euo pipefail
    cargo build --release
    sccache --show-stats || true
    cp -a /target/release/silvapi /work/silvapi/dist/Silvapi-linux-x86_64
    cp -a /target/release/silvapi /work/silvapi/silvapi-linux"

echo "Built: $dist_dir/Silvapi-linux-x86_64"
echo "Convenience copy: $repo_dir/silvapi-linux"
echo
echo "Dynamic runtime libraries:"
ldd "$repo_dir/silvapi-linux" | sed -n '1,120p'
