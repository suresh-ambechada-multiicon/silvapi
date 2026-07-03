#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workspace_dir="$(cd "$repo_dir/.." && pwd)"
dist_dir="$repo_dir/dist"
target_dir="$repo_dir/.cache/target-linux"

mkdir -p "$dist_dir" "$target_dir"

docker build -f "$repo_dir/Dockerfile.compat" -t silvapi-build:bullseye "$workspace_dir"
docker run --rm \
  -v "silvapi-cargo-registry:/cargo/registry" \
  -v "silvapi-cargo-git:/cargo/git" \
  -v "$target_dir:/work/silvapi/target" \
  -v "$workspace_dir:/work" \
  -w /work/silvapi \
  silvapi-build:bullseye \
  bash -c "cargo build --release && cp -a target/release/silvapi /work/silvapi/dist/Silvapi-linux-x86_64 && cp -a target/release/silvapi /work/silvapi/silvapi-linux"

echo "Built: $dist_dir/Silvapi-linux-x86_64"
echo "Convenience copy: $repo_dir/silvapi-linux"
echo
echo "Dynamic runtime libraries:"
ldd "$repo_dir/silvapi-linux" | sed -n '1,120p'
