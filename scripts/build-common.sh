#!/usr/bin/env bash
# Shared build configuration sourced by the release build scripts.
#
# Provides:
#   - Persistent cache directories under .cache/ (cargo registry/git, sccache,
#     per-target dirs) so nothing is re-downloaded or fully recompiled between
#     builds.
#   - Memory/CPU-aware settings so the build works on low-end / low-RAM
#     machines instead of OOM-ing on the aggressive fat-LTO release profile.
#
# Tunables (env):
#   SILVAPI_LOW_END=1   force the low-end profile (thin LTO, more codegen units)
#   SILVAPI_JOBS=N      override the number of parallel compile jobs
#   SILVAPI_FULL_LTO=1  force fat LTO even on a low-RAM machine (may OOM)

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cache_dir="$repo_dir/.cache"

# --- Shared, persistent caches -------------------------------------------------
export CACHE_CARGO_REGISTRY="$cache_dir/cargo-registry"
export CACHE_CARGO_GIT="$cache_dir/cargo-git"
export CACHE_SCCACHE="$cache_dir/sccache"
mkdir -p "$CACHE_CARGO_REGISTRY" "$CACHE_CARGO_GIT" "$CACHE_SCCACHE"

# --- Detect host resources -----------------------------------------------------
host_cores="$(nproc 2>/dev/null || echo 2)"
host_ram_gb="$(free -g 2>/dev/null | awk '/^Mem:/ {print $2}')"
[ -z "${host_ram_gb:-}" ] && host_ram_gb=8
# `free -g` floors to 0 on <1GB and rounds down; treat 0 as 1.
[ "$host_ram_gb" -lt 1 ] && host_ram_gb=1

# Low-end if forced, or if the machine has little RAM.
low_end=0
if [ "${SILVAPI_LOW_END:-0}" = "1" ] || [ "$host_ram_gb" -le 7 ]; then
  low_end=1
fi
# Exported so the orchestrator (build-release.sh) can build sequentially
# instead of running Linux + Windows in parallel (which doubles peak RAM).
export SILVAPI_IS_LOW_END="$low_end"

# --- Parallelism ---------------------------------------------------------------
# Cap jobs by RAM: heavy crates (gpui, tree-sitter, etc.) can each need ~1.5 GB
# during codegen, so allowing one job per core on a low-RAM box OOMs.
if [ -n "${SILVAPI_JOBS:-}" ]; then
  build_jobs="$SILVAPI_JOBS"
else
  ram_jobs=$(( host_ram_gb / 2 ))
  [ "$ram_jobs" -lt 1 ] && ram_jobs=1
  build_jobs="$host_cores"
  [ "$ram_jobs" -lt "$build_jobs" ] && build_jobs="$ram_jobs"
fi
export BUILD_JOBS="$build_jobs"

# --- Release profile overrides (via cargo env, no Cargo.toml edits) ------------
# Fat LTO + codegen-units=1 gives the smallest binary but needs a lot of RAM at
# link time and is very slow — impractical on low-end machines. On a low-RAM
# host, fall back to thin LTO with more codegen units: far less memory, much
# faster, only a slightly larger binary.
if [ "$low_end" = "1" ] && [ "${SILVAPI_FULL_LTO:-0}" != "1" ]; then
  export CARGO_PROFILE_RELEASE_LTO="thin"
  export CARGO_PROFILE_RELEASE_CODEGEN_UNITS="16"
  profile_note="low-end (thin LTO, codegen-units=16)"
else
  profile_note="default (fat LTO, codegen-units=1)"
fi

# --- Docker argument helpers ---------------------------------------------------
# Common `-e` flags: sccache wrapper + cache dir, job cap, profile overrides.
common_docker_env=(
  -e "RUSTC_WRAPPER=sccache"
  -e "SCCACHE_DIR=/sccache"
  -e "CARGO_BUILD_JOBS=$BUILD_JOBS"
  # Fetch git deps with the git CLI (installed in the images) instead of
  # libgit2 — libgit2 often fails to retrieve a pinned `rev = <sha>` from a
  # fresh cache ("revision not found"). The git CLI fetches it reliably.
  -e "CARGO_NET_GIT_FETCH_WITH_CLI=true"
)
[ -n "${CARGO_PROFILE_RELEASE_LTO:-}" ] && common_docker_env+=(-e "CARGO_PROFILE_RELEASE_LTO=$CARGO_PROFILE_RELEASE_LTO")
[ -n "${CARGO_PROFILE_RELEASE_CODEGEN_UNITS:-}" ] && common_docker_env+=(-e "CARGO_PROFILE_RELEASE_CODEGEN_UNITS=$CARGO_PROFILE_RELEASE_CODEGEN_UNITS")

# Common `-v` flags: persistent sccache cache. The `:z` suffix relabels the
# bind mount for SELinux (Fedora/RHEL) — without it the container gets
# "Permission denied" on the host directory. It's a harmless no-op on distros
# without SELinux.
common_docker_volumes=(
  -v "$CACHE_SCCACHE:/sccache:z"
)

print_build_plan() {
  echo "Build plan:"
  echo "  host: ${host_cores} cores, ${host_ram_gb} GB RAM"
  echo "  jobs: ${BUILD_JOBS}"
  echo "  profile: ${profile_note}"
  echo "  caches: $cache_dir/{cargo-registry,cargo-git,sccache,target-*}"
}
