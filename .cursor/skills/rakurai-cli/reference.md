# Rakurai CLI Reference

Command flags and auth checks. Source: `cli/src/bin/rakurai_activation_cli.rs`, `cli/README.md`.

---

## Global Options

```
rakurai-activation [OPTIONS] --program-id <PROGRAM_ID> <COMMAND>

-k, --keypair <PATH>     default: ~/.config/solana/id.json
-u, --url <URL>          default: t; monikers: m, t, d, l
-p, --program-id <ID>    required
```

---

## Operator Commands

### `init`

| Flag | Alias | Required | Default |
|------|-------|----------|---------|
| `--vote_pubkey` | `-v` | yes | — |
| `--block_reward_commission_bps` | `-c`, `--commission_bps` | no | 10000 |

Auth: signer = vote node pubkey. `reconfirm_commission` if bps < 10000.

### `scheduler-control`

| Flag | Alias | Required | Default |
|------|-------|----------|---------|
| `--identity_pubkey` | `-i` | yes | — |
| `--disable_scheduler` | `-d` | no | enable |
| `--hash` | `-s` | no | — |

Auth: identity OR `client_authority`. `grant_approval` ix arg = `disable_scheduler` CLI field.

### `update-commission`

| Flag | Alias | Required |
|------|-------|----------|
| `--identity_pubkey` | `-i` | yes |
| `--block_reward_commission_bps` | `-c`, `--commission_bps` | yes |

Auth: validator OR client. Aborts if unchanged.

### `show`

| Flag | Alias | Required |
|------|-------|----------|
| `--identity_pubkey` | `-i` | yes |

Read-only fetch; no signer required.

---

## Hidden Admin Commands

### `init-config`

| Flag | Alias | Required |
|------|-------|----------|---------|
| `--commission_bps` | `-c` | yes |
| `--commission_account` | `-a` | yes |
| `--authority` | `-b` | yes (client) |
| `--config_authority` | `-x` | yes |

### `show-config`

No extra args.

### `close`

| Flag | Alias | Required |
|------|-------|----------|---------|
| `--identity_pubkey` | `-i` | yes |

Auth: `client_authority`.

---

## lib.rs Helpers

```rust
pub const MAX_COMMISSION_BPS: u16 = 10_000;
pub fn parse_pubkey(s: &str) -> Result<Pubkey, String>;
pub fn normalize_to_url_if_moniker(url_or_moniker: &str) -> Result<String, String>;
pub fn validate_commission(val: &str) -> Result<u16, String>;
pub fn parse_keypair(path: &str) -> Result<Arc<Keypair>, Box<dyn std::error::Error>>;
pub fn reconfirm_commission(bps: u16) -> Result<(), Box<dyn std::error::Error>>;
pub fn get_activation_account(...) -> Result<RakuraiActivationAccount, ...>;
pub fn get_activation_config_account(...) -> Result<RakuraiActivationConfigAccount, ...>;
pub fn display_activation_account(...);
pub fn display_activation_config_account(...);
pub fn get_node_pubkey_from_vote_account(...) -> Result<Pubkey, ...>;
pub fn sign_and_send_transaction(...) -> Result<(), ...>;
```

---

## New Binary Template

```toml
[[bin]]
name = "rakurai-partner-reward-settlement"
path = "src/bin/rakurai_partner_reward_settlement_cli.rs"

[dependencies]
reward_distribution = { path = "../programs/reward_distribution" }
```

Anchor ix → Solana ix: map `ix.accounts` to `AccountMeta` with `Pubkey::new_from_array(a.pubkey.to_bytes())`, then `Instruction::new_with_bytes(program_id, &ix.data, acct_metas)`.

---

## Tip Manager & RCA CLI Accounts (reference)

**change_tip_receiver_v1**: legacy TCA (`REVENUE_SHARE`); CPI `record_revenue`; drain commission from TM global (previous leader), sync global from new TCA after. Derive: `derive_rakurai_tip_collection_address`.

**change_tip_receiver_v2**: TCAV1 (`REVENUE_SHARE_V1`); CPI `record_revenue_v1`; drain commission from TM global (previous leader), sync global from new TCAV1 after. Derive: `derive_rakurai_tip_collection_v1_address`.

**transfer_staker_rewards** (RD): validator_vote_account, client_commission_account, reward_collection_account, system_program, validator identity signer.

RCA credit after drain: separate off-chain lamport transfer using `derive_reward_collection_account_address(rd_id, vote, epoch)`.
