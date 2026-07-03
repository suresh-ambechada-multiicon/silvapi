#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
log_dir="$repo_dir/dist/logs"

mkdir -p "$log_dir"

run_build() {
  name="$1"
  script="$2"
  log_file="$log_dir/$name.log"

  echo "Starting $name build..."
  if "$script" 2>&1 | sed -u "s/^/[$name] /" | tee "$log_file"; then
    echo "$name build finished. Log: $log_file"
    return 0
  fi

  echo "$name build failed. Last log lines:"
  tail -n 80 "$log_file" || true
  return 1
}

run_build linux "$repo_dir/scripts/build-compatible-release.sh" &
linux_pid="$!"

run_build windows "$repo_dir/scripts/build-windows-exe.sh" &
windows_pid="$!"

linux_status=0
windows_status=0

wait "$linux_pid" || linux_status="$?"
wait "$windows_pid" || windows_status="$?"

if [ "$linux_status" -ne 0 ] || [ "$windows_status" -ne 0 ]; then
  echo
  echo "Release build failed."
  echo "Linux status: $linux_status"
  echo "Windows status: $windows_status"
  exit 1
fi

echo
echo "Release artifacts:"
ls -lh "$repo_dir/dist"
