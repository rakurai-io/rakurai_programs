---
name: rakurai-cli
description: >-
  Rakurai CLI crate (rakurai_cli lib, rakurai-activation binary). Use when building
  or extending Rakurai CLIs, adding reward-distribution or tip-manager binaries,
  shared RPC/keypair helpers, or operator command patterns.
---

# Rakurai CLI

Crate at `cli/` (`rakurai_cli`). Shared lib in `cli/src/lib.rs`; binaries in `cli/src/bin/`.

**Current**: `rakurai-activation`, `rakurai-partner-reward-settlement`. New binaries should follow patterns here plus program skills below.

**Flags**: [reference.md](reference.md).

---

## Layout & Build

```
cli/Cargo.toml, README.md, ACTIVATION.md, PARTNER_REWARD_SETTLEMENT.md,
src/lib.rs, src/bin/rakurai_activation_cli.rs,
src/bin/rakurai_partner_reward_settlement_cli.rs
```

```bash
cargo build --release -p rakurai_cli
# → target/release/rakurai-activation
# → target/release/rakurai-partner-reward-settlement
```

---

## Shared Library (`lib.rs`)

| Function | Purpose |
|----------|---------|
| `parse_pubkey` | Pubkey parse + error |
| `normalize_to_url_if_moniker` | `m`/`t`/`d`/`l` → RPC URL |
| `validate_commission` | u16, cap ≤ 10_000 |
| `parse_keypair` | Tilde path → `Arc<Keypair>` |
| `sign_and_send_transaction` | Blockhash, sign, confirm |
| `get_node_pubkey_from_vote_account` | Vote bytes `[4..36]` |
| `reconfirm_commission` | Stdin confirm if bps < 100% |
| `get_activation_account` / `get_activation_config_account` | Fetch + deserialize |
| `display_activation_account` / `display_activation_config_account` | Colored stdout |

`MAX_COMMISSION_BPS = 10_000`. Add program-specific `get_*` / `display_*` for new CLIs.

---

## Global Conventions

| Flag | Default | Notes |
|------|---------|-------|
| `--keypair` `-k` | `~/.config/solana/id.json` | Write signer |
| `--url` `-u` | `t` (testnet) | Monikers: m, t, d, l |
| `--program-id` `-p` | **required** | Cluster-specific |

| Program | Mainnet | Testnet | Localnet |
|---------|---------|---------|----------|
| rakurai_activation | `rAKACC6Qw8HYa87ntGPRbfYEMnK2D9JVLsmZaKPpMmi` | `pmQHMpnpA534JmxEdwY3ADfwDBFmy5my3CeutHM2QTt` | `CuvTfdaxcDbvvtACkXrW2j69YeQbWuNnwg9FefYDrSug` |
| reward_distribution | `RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB` | `A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB` | `CtVB7ze4Kz2iUHGrWLWY9EG5Au1erbRCMvKTWFuKv8wq` |
| rakurai_tip_manager | `rKtiPTD7WuCdEEQ2JXWgAmZHHL9iZLc3niCXwtS7wSH` | `4qRZaFzf7MvgfBTCP9grb69cCST8UmKHPtkpGAgkJosD` | `6z4rnNKVzSYBxqfshk1QZFgJv17KjZoirFhpWSjqQMfu` |

`declare_id!` = testnet in each program; pass correct `--program-id` per cluster.

---

## `rakurai-activation` Binary

| Subcommand | On-chain ix |
|------------|-------------|
| `init` | `initialize_rakurai_activation_account` (`-v` vote) |
| `scheduler-control` | `update_rakurai_activation_approval` (`-d` disable, `--hash`) |
| `update-commission` | `update_rakurai_activation_commission` |
| `show` | read RAA |
| `init-config` / `show-config` / `close` | admin (hidden) |

**Flow**: parse → keypair → RpcClient → derive PDAs → pre-flight (vote→identity) → SDK ix → `AccountMeta` conversion → `sign_and_send_transaction`.

```bash
cargo run --bin rakurai-activation -- -p pmQHMpnpA534JmxEdwY3ADfwDBFmy5my3CeutHM2QTt -u t init -v <VOTE> -c 9500
```

---

## Adding a New Binary

1. `[[bin]]` + path dep in `cli/Cargo.toml`
2. Copy activation `Cli` struct (global `-k`/`-u`/`-p`)
3. Subcommands → operator workflows, not every on-chain ix
4. Reuse `lib.rs`; add `get_*`/`display_*` as needed

**Suggested v1 — Partner Reward Settlement CLI** (`rakurai-partner-reward-settlement`): `get-account`, `get-pending-record`, `get-all-pending-records`, `transfer` for TCA/MCA.

**Suggested v1 — tip_manager**: `show-config`, `claim-tips`, `change-tip-receiver`, `change-client`. Optional `--reward-distribution-program-id` only if CLI also credits RCA off-chain.

---

## Related Skills

[rakurai-activation](../rakurai-activation/SKILL.md) · [reward-distribution](../reward-distribution/SKILL.md) · [rakurai-tip-manager](../rakurai-tip-manager/SKILL.md)
