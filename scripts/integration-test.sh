#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export BPF_OUT_DIR="$ROOT/target/deploy"
export SBF_OUT_DIR="$ROOT/target/deploy"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-1.93.1}"

echo "Syncing program IDs with deploy keypairs..."
anchor keys sync

echo "Building on-chain programs..."
export CARGO_TARGET_DIR="$ROOT/target"
anchor build --no-idl

# Ensure deploy/ has freshly linked program .so files (release/ or deps/ fallback).
mkdir -p "$BPF_OUT_DIR"
for prog in rakurai_activation reward_distribution rakurai_tip_manager; do
  src="$ROOT/target/sbpf-solana-solana/release/${prog}.so"
  if [[ ! -f "$src" ]]; then
    src="$ROOT/target/sbpf-solana-solana/release/deps/${prog}.so"
  fi
  if [[ -f "$src" ]]; then
    cp -f "$src" "$BPF_OUT_DIR/${prog}.so"
  fi
done

for so in rakurai_activation reward_distribution rakurai_tip_manager; do
  if [[ ! -f "$BPF_OUT_DIR/${so}.so" ]]; then
    echo "error: missing $BPF_OUT_DIR/${so}.so (anchor build failed?)" >&2
    exit 1
  fi
done

echo "Running integration tests..."
cargo test -p rakurai_integration -- --nocapture
