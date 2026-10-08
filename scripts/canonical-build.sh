#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
case "${1:-}" in
  guest|ckb) test "$#" = 1 ;;
  rebuild-verifier)
    test "$#" = 2
    test "$(dirname "$2")" = .local
    case "$(basename "$2")" in verifier-rebuild.*) ;; *) exit 2 ;; esac
    test -d "$2" && ! test -L "$2"
    ;;
  *) echo 'usage: scripts/canonical-build.sh guest|ckb|rebuild-verifier .local/verifier-rebuild.DIRECTORY' >&2; exit 2 ;;
esac
root="$PWD"
cargo_cache="${CARGO_HOME:-$HOME/.cargo}"
container_root=/workspace
container_cargo=/usr/local/cargo
if test "$1" = guest; then
  # Panic locations are embedded in this admitted guest. Preserve both original
  # source roots so the complete ELF and program key reproduce without a repin.
  container_root=/home/arthur/RustRoverProjects/AgoraSeal
  container_cargo=/home/arthur/.cargo
fi
docker build --platform linux/amd64 -t agoraseal-canonical:rust-1.97.1 \
  -f docker/Dockerfile.canonical docker
docker run --rm --platform linux/amd64 \
  -v "$root:$container_root" \
  -v "$cargo_cache/registry:$container_cargo/registry:ro" \
  -v "$cargo_cache/git:$container_cargo/git:ro" \
  -e "CARGO_HOME=$container_cargo" \
  -w "$container_root" agoraseal-canonical:rust-1.97.1 bash scripts/canonical-inside.sh "$@"
