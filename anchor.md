

## How to build and deploy programs using Anchor
This repository contains the necessary commands and steps for building, deploying, upgrading, and closing Solana programs. The commands utilize the Anchor framework for managing the deployment and interaction with Solana-based programs.

## Setup & Installation

1. **Install Anchor**: 
   Make sure you have Anchor installed. If not, follow the official instructions to install it: 
   [Install Anchor](https://project-serum.github.io/anchor/getting-started/installation.html).

2. **Install Solana CLI**: 
   Ensure that the Solana CLI is installed. Follow the official installation guide: 
   [Solana CLI Installation](https://docs.solana.com/cli/install-solana-cli-tools).

## Commands Overview

### 1. Build the Anchor Program
To build the program, use the following command:

```bash
anchor build
```

### 1. Sync the Wallet Keys
Synchronize Program ID and rebuild:

```bash
anchor keys sync
anchor build
```
### 3. Deploy the Program
To deploy the program to the Solana cluster, use the following command. Make sure to specify the correct wallet and program files:

```bash
anchor deploy --provider.cluster t --provider.wallet ~/.config/solana/id.json
```
This will deploy your program to the Solana testnet (t cluster) using the wallet at `~/.config/solana/id.json`.

### 4. Upgrade the Program
To upgrade an existing deployed program, use the following command. Replace the program-id with your deployed program ID and specify the path to the new program .so file:

```bash
anchor upgrade --program-id <DeloyedProgramID> ./target/deploy/block_reward_distribution.so --provider.cluster t --provider.wallet ~/.config/solana/id.json
```
This upgrades the program at the specified program-id to the new program located at ./target/deploy/block_reward_distribution.so.

### 5. Close the Program
To close a Solana program, use the following command. You must provide the program's public key and the path to your wallet's keypair file:

```bash
solana program close 4wyjfWEX6746eoepd37Gb6KcPpLpkJhe4CqWzerLfpCB --keypair ~/.config/solana/id.json -ut --bypass-warning
```
This command will close the program and reclaim the funds.

## Building the IDL (Anchor 0.30.1)

> **Anchor version:** `anchor-cli 0.30.1`

To generate the IDL for a program:

```bash
anchor idl build --program-name rakurai_activation -o ./idl/rakurai_activation.json
```

### Fixing the `source_file` build error

On Anchor `0.30.1`, `anchor idl build` may fail while compiling `anchor-syn` with:

```
error[E0599]: no method named `source_file` found for struct `proc_macro2::Span` in the current scope
   --> .../anchor-syn/.../idl/defined.rs:499:66
```

**Cause:** This is a Rust compile error inside `anchor-syn` itself (not a missing `/// CHECK:` doc comment). Anchor 0.30.1 calls `proc_macro2::Span::call_site().source_file().path()`, but `proc-macro2 >= 1.0.95` removed the semver-exempt `source_file()` method (it was replaced by `.file()` / `.local_file()` to track a nightly compiler change made on 2025-04-16).

**Fix:** Three things must align for Anchor 0.30.1 — pin `proc-macro2` to `1.0.94`, use a nightly from *before* 2025-04-16, and pass the `procmacro2_semver_exempt` cfg.

1. Pin `proc-macro2` back to the last version that has `source_file()`:

```bash
cargo update -p proc-macro2 --precise 1.0.94
```

2. Install a nightly toolchain from before the 2025-04-16 API removal (so `proc-macro2 1.0.94` itself compiles):

```bash
rustup toolchain install nightly-2025-04-14 --profile minimal
```

3. Build the IDL with that toolchain and the semver-exempt flag (required to expose `source_file()`):

```bash
RUSTUP_TOOLCHAIN=nightly-2025-04-14 RUSTFLAGS='--cfg procmacro2_semver_exempt' \
  anchor idl build --program-name rakurai_activation -o ./idl/rakurai_activation.json
```

> **Note:** Keep the `proc-macro2 = "=1.0.94"` pin so a future `cargo update` doesn't bump it back and reintroduce the error. The cleaner long-term fix is upgrading to Anchor `0.31.1+`, which uses the new `.file()` API and works with current `proc-macro2` and recent nightlies.