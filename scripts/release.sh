#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
[[ $(uname -m) == x86_64 ]] || { echo 'This release target requires an x86_64 builder.' >&2; exit 1; }
docker build --platform linux/amd64 --target builder -t vitrilyr-release-build \
    -f "$root/packaging/Dockerfile" "$root"
container=$(docker create vitrilyr-release-build /bin/true)
trap 'docker rm "$container" >/dev/null' EXIT
mkdir -p "$root/dist"
docker cp "$container:/out/." "$root/dist/"
printf 'Release files: %s/dist\n' "$root"
