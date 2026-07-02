#!/usr/bin/env bash
#
# build-idl.sh - Generate Anchor IDLs without keeping the `idl-build`
# feature in the committed Cargo.toml.
#
# Why: keeping `idl-build` defined/enabled in Cargo.toml can break the
# normal `anchor build` (on-chain .so) flow. So this script ADDS the
# `idl-build` feature to the program's Cargo.toml ONLY while generating
# the IDL, then removes it again, restoring each Cargo.toml to its exact
# original contents (net zero change to git).
#
# Usage:
#   scripts/build-idl.sh                       # auto-detect crates with #[program]
#   scripts/build-idl.sh all                   # every program except vote_state
#   scripts/build-idl.sh reward_distribution   # build IDL for specific program(s)
#
set -euo pipefail

# --- Toolchain requirements for Anchor 0.30.1 IDL builds (see anchor.md) ---
# proc-macro2 must be pinned to 1.0.94 in Cargo.lock and a pre-2025-04-16
# nightly is required so that `source_file()` still compiles.
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-nightly-2025-04-14}"
export RUSTFLAGS="${RUSTFLAGS:---cfg procmacro2_semver_exempt}"

# The feature line that gets temporarily injected.
IDL_FEATURE_LINE='idl-build = ["anchor-lang/idl-build"]'
# proc-macro2 must be pinned here for Anchor 0.30.1's IDL builder to compile.
PROC_MACRO2_PIN="1.0.94"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROGRAMS_DIR="$REPO_ROOT/programs"

# Track backups so we can always restore, even on error / Ctrl-C.
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

# Ensure Cargo.lock pins proc-macro2 to a version that still exposes
# `source_file()`. Backs up Cargo.lock first so restore_all reverts it,
# leaving no net change to the lockfile.
pin_proc_macro2() {
  local lock="$REPO_ROOT/Cargo.lock"
  [ -f "$lock" ] || return 0
  cp "$lock" "$lock.idlbak"
  BACKUPS+=("$lock.idlbak")

  local have
  have="$(grep -A1 'name = "proc-macro2"' "$lock" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)"
  if [ "$have" = "$PROC_MACRO2_PIN" ]; then
    echo ">> proc-macro2 already pinned to $PROC_MACRO2_PIN"
    return 0
  fi
  echo ">> pinning proc-macro2 $have -> $PROC_MACRO2_PIN"
  ( cd "$REPO_ROOT" && cargo update -p proc-macro2 --precise "$PROC_MACRO2_PIN" )
}

# Add the `idl-build` feature definition to a Cargo.toml, in place.
# No-op if it's already defined. Backs up the original first.
add_idl_feature() {
  local cargo="$1"
  cp "$cargo" "$cargo.idlbak"
  BACKUPS+=("$cargo.idlbak")

  if grep -qE '^[[:space:]]*idl-build[[:space:]]*=' "$cargo"; then
    return 0  # already defined; backup still ensures a clean restore
  fi

  if grep -qE '^\[features\]' "$cargo"; then
    # Insert right after the [features] header.
    awk -v line="$IDL_FEATURE_LINE" '
      !done && /^\[features\]/ { print; print line; done = 1; next }
      { print }
    ' "$cargo.idlbak" > "$cargo"
  else
    # No [features] table: append one at the end of the file.
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

  echo ">> [$prog] adding idl-build feature to Cargo.toml"
  add_idl_feature "$cargo"

  mkdir -p "$out_dir"
  echo ">> [$prog] building IDL (toolchain=$RUSTUP_TOOLCHAIN)"
  ( cd "$PROGRAMS_DIR/$prog" && \
    anchor idl build --program-name "$prog" -o "./idl/$prog.json" )

  echo ">> [$prog] removing idl-build feature (restoring Cargo.toml)"
  if [ -f "$cargo.idlbak" ]; then
    mv -f "$cargo.idlbak" "$cargo"
  fi
  echo ">> [$prog] done -> programs/$prog/idl/$prog.json"
}

# Programs to skip when building "all".
EXCLUDE=("vote_state")

is_excluded() {
  local p="$1" e
  for e in "${EXCLUDE[@]}"; do
    [ "$p" = "$e" ] && return 0
  done
  return 1
}

# Determine which programs to build.
#   (no args)      -> auto-detect crates that declare #[program]
#   all            -> every crate under programs/ except EXCLUDE (e.g. vote_state)
#   <name> [<name>]-> the named program(s)
PROGRAMS=()
if [ "$#" -eq 0 ]; then
  # Auto-detect: any crate under programs/ whose source declares #[program].
  for d in "$PROGRAMS_DIR"/*/; do
    prog="$(basename "$d")"
    is_excluded "$prog" && continue
    if grep -rqE '^\s*#\[program\]' "$d/src" 2>/dev/null; then
      PROGRAMS+=("$prog")
    fi
  done
elif [ "$1" = "all" ] || [ "$1" = "--all" ] || [ "$1" = "-a" ]; then
  # Every program directory under programs/ except the excluded ones.
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
pin_proc_macro2
for prog in "${PROGRAMS[@]}"; do
  build_one "$prog"
done

echo "All done."
