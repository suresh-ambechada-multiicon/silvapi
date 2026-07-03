#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workspace_dir="$(cd "$repo_dir/.." && pwd)"

docker build -f "$repo_dir/Dockerfile.compat" -t silvapi-build:bookworm "$workspace_dir"
docker run --rm \
  -v "silvapi-cargo-registry:/cargo/registry" \
  -v "silvapi-cargo-git:/cargo/git" \
  -v "silvapi-target:/work/silvapi/target" \
  -v "$workspace_dir:/work" \
  -w /work/silvapi \
  silvapi-build:bookworm \
  bash -c "cargo build --release && cp -a target/release/silvapi /work/silvapi/silvapi-linux"

echo "Built: $repo_dir/silvapi-linux"
ldd "$repo_dir/silvapi-linux" | sed -n '1,120p'
