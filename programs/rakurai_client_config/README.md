# Rakurai Client Config Program

On-chain configuration for **Rakurai validators**: where the scheduler sends **block-engine** bundles, which **post-pack confirmation (P2C)** endpoints are registered, and how **virtual priority** is applied per account. Config is stored in PDAs (global defaults + per-vote overrides) that the validator node reads at runtime.

➤ For more details, refer to the [IDL file](./idl/rakurai_client_config.json).

---

## 1. Deployed program ID

- **Mainnet / Testnet / Localnet (current):** [4uGNMjJFxgE3TfEiPmSpvfwYah12QZbaWWZDJqZvA9F4](https://solscan.io/account/4uGNMjJFxgE3TfEiPmSpvfwYah12QZbaWWZDJqZvA9F4)

---

## 2. What this program configures

Each validator vote account can have live on-chain settings for three Rakurai scheduler surfaces. All three live inside one versioned payload (`ConfigV1`) on the global and per-vote PDAs:

| Config section | What it controls | Typical use |
|----------------|------------------|-------------|
| **`block_engine`** | Named endpoint groups for the **block engine** — URLs plus bundle rate limits (`max_bundles`, `period_ms`, `max_bundle_burst`) | Where searchers / partners submit bundles and how fast they may send |
| **`p2c`** | Named endpoint groups for **post-pack confirmation (P2C)** — gRPC URLs only | Where post-pack consumers receive transaction updates from the validator ([post-pack docs](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/post_pack_confirmations)) |
| **`virtual_priority`** | Named groups mapping **account pubkey → priority multiplier** (`f64`) | Scheduler-side virtual priority for specific accounts (higher value = more priority) |

Each section is a list of **named sets** (`Uuid`, up to 32 bytes). A set contains one or more entries (URLs for BE/P2C, pubkey+value pairs for VP). Entry names are how global and per-validator layers are merged (see §3).

This program stores **configuration only** — not tip accounts, P2C subscription escrows, or other revenue PDAs. It tells the validator *which endpoints and priority rules apply* for that vote.

---

## 3. Account layers (global / validator / proposal)

Rakurai ops set network-wide defaults; each validator can override per vote account. Operators can propose changes; the manager approves or rejects.

| Layer | PDA seed | Who writes | Purpose |
|-------|----------|------------|---------|
| **Global** | `global-validator-config` | **Manager** | Default block-engine, P2C, and virtual-priority sets for all validators |
| **Validator** | `validator-config` + vote | **Manager** | Live per-vote config (copied from global on init; manager can replace) |
| **Proposal** | `validator-proposal` + vote | **Operator** → **Manager** | Draft overlay; `approve_proposal` copies into live validator PDA |

**Effective config** for a vote = **global ∪ validator** (validator PDA optional), merged **by set name**. If no validator PDA exists, effective config is global only. The CLI shows this with `rakurai-client-config union`.

---

## 4. Config payload (`ConfigV1`)

Versioned enum `Config::V1(ConfigV1)`. Accounts **realloc** on every init/update as sets grow or shrink.

| Section | JSON / on-chain shape | Validation |
|---------|----------------------|------------|
| `block_engine.sets[]` | `name` + `url[]` with quota fields | Every URL must be non-empty |
| `p2c.sets[]` | `name` + `url[]` | Every URL must be non-empty |
| `virtual_priority.sets[]` | `name` + `key` (pubkey) + `value` (f64) | Pubkeys must be valid |

Example JSON: [`cli/examples/validator_config.json`](../../cli/examples/validator_config.json) (global baseline), [`cli/examples/validator_config_overlay.json`](../../cli/examples/validator_config_overlay.json) (per-vote overlay).

---

## 5. Accounts and auth

| Instruction | Signer | Effect |
|-------------|--------|--------|
| `init_global` | manager (becomes stored manager) | Create singleton global PDA |
| `update_global` | manager | Replace global payload; realloc |
| `close_global` | manager | Close global PDA; rent → manager |
| `init_validator` | manager | Create per-vote PDA; copies global config; sets `operator` |
| `update_validator` | manager | Replace live validator payload; realloc |
| `set_operator` | manager | Change who may propose |
| `close_validator` | manager | Close validator PDA |
| `init_proposal` | operator | Create draft PDA for vote |
| `update_proposal` | operator | Replace draft; realloc |
| `approve_proposal` | manager | Copy draft → live validator; close proposal (rent → operator) |
| `reject_proposal` | manager | Close proposal without changing live config |

---

## 6. Proposal flow

1. Manager `init_validator --operator <OP>` (live config = global snapshot).
2. Operator `proposal submit` with JSON overlay (live unchanged).
3. Manager `proposal approve` (promote draft) or `proposal reject` (discard draft).

---

## 7. SDK

Off-chain helpers live in `src/sdk/`:

- PDA derivation: `derive_global_config_address`, `derive_validator_config_address`, `derive_validator_proposal_address`
- Instruction builders: `init_global_ix`, `update_validator_ix`, `approve_proposal_ix`, …
- Client merge: `union_configs(global, validator)`

---

## 8. CLI

See [Client Config CLI](../../cli/client_config.md) (`rakurai-client-config`).

Example JSON: [`cli/examples/validator_config.json`](../../cli/examples/validator_config.json), [`cli/examples/validator_config_overlay.json`](../../cli/examples/validator_config_overlay.json).

---

## 9. Security

Embedded [`security_txt`](https://github.com/neodyme8/solana-security-txt) in the program binary (see `src/lib.rs`). Report issues via contacts listed there and at [rakurai.io](https://rakurai.io/).
