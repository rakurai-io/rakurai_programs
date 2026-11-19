#!/usr/bin/env bash
#
# build-idl.sh - Write Anchor IDLs for one or more programs.
#
# Anchor 1.2.0 reads the committed `idl-build` feature and rust-toolchain.toml
# (Rust 1.89.0). Do not pin an old nightly or proc-macro2; that was only
# required for Anchor 0.30.1.
#
# Usage:
#   scripts/build-idl.sh                       # crates that declare #[program]
#   scripts/build-idl.sh all                   # every program except vote_state
#   scripts/build-idl.sh reward_distribution   # one or more program names
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROGRAMS_DIR="$REPO_ROOT/programs"
IDL_FEATURE_LINE='idl-build = ["anchor-lang/idl-build"]'

# Host IDL build must follow rust-toolchain.toml, not a leftover nightly.
unset RUSTUP_TOOLCHAIN
unset RUSTFLAGS

BACKUPS=()
restore_all() {
  local b
  for b in "${BACKUPS[@]:-}"; do
    if [ -n "${b:-}" ] && [ -f "$b" ]; then
      mv -f "$b" "${b%.idlbak}"
    fi
  done
}
trap restore_all EXIT INT TERM

# Older checkouts may lack idl-build. Add it only for this run, then restore.
ensure_idl_feature() {
  local cargo="$1"
  if grep -qE '^[[:space:]]*idl-build[[:space:]]*=' "$cargo"; then
    return 0
  fi

  cp "$cargo" "$cargo.idlbak"
  BACKUPS+=("$cargo.idlbak")
  echo ">> temporarily adding idl-build to $cargo"

  if grep -qE '^\[features\]' "$cargo"; then
    awk -v line="$IDL_FEATURE_LINE" '
      !done && /^\[features\]/ { print; print line; done = 1; next }
      { print }
    ' "$cargo.idlbak" > "$cargo"
  else
    cp "$cargo.idlbak" "$cargo"
    printf '\n[features]\n%s\n' "$IDL_FEATURE_LINE" >> "$cargo"
  fi
}

build_one() {
  local prog="$1"
  local cargo="$PROGRAMS_DIR/$prog/Cargo.toml"
  local out_dir="$PROGRAMS_DIR/$prog/idl"

  if [ ! -f "$cargo" ]; then
    echo "!! skip $prog: $cargo not found"
    return 1
  fi

  ensure_idl_feature "$cargo"
  mkdir -p "$out_dir"
  echo ">> [$prog] anchor idl build"
  (
    cd "$REPO_ROOT"
    anchor idl build --program-name "$prog" -o "$out_dir/$prog.json"
  )
  echo ">> [$prog] done -> programs/$prog/idl/$prog.json"
}

EXCLUDE=("vote_state")

is_excluded() {
  local p="$1" e
  for e in "${EXCLUDE[@]}"; do
    [ "$p" = "$e" ] && return 0
  done
  return 1
}

PROGRAMS=()
if [ "$#" -eq 0 ]; then
  for d in "$PROGRAMS_DIR"/*/; do
    prog="$(basename "$d")"
    is_excluded "$prog" && continue
    if grep -rqE '^\s*#\[program\]' "$d/src" 2>/dev/null; then
      PROGRAMS+=("$prog")
    fi
  done
elif [ "$1" = "all" ] || [ "$1" = "--all" ] || [ "$1" = "-a" ]; then
  for d in "$PROGRAMS_DIR"/*/; do
    prog="$(basename "$d")"
    is_excluded "$prog" && { echo ">> skipping excluded program: $prog"; continue; }
    PROGRAMS+=("$prog")
  done
else
  PROGRAMS=("$@")
fi

if [ "${#PROGRAMS[@]}" -eq 0 ]; then
  echo "No anchor programs found to build."
  exit 1
fi

echo "Building IDL for: ${PROGRAMS[*]}"
for prog in "${PROGRAMS[@]}"; do
  build_one "$prog"
done

echo "All done."
