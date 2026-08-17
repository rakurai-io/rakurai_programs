# Validator Config CLI (`rakurai-validator-config`)

Manage on-chain **block-engine**, **post-pack confirmation (P2C)**, and **virtual-priority** settings for Rakurai validators — global defaults, per-vote live config, and operator proposals.

**Audience:** Rakurai ops (manager) and validator operators (operator for proposals).  
**Program:** `rakurai_validator_config` — see [program README](../programs/rakurai_validator_config/README.md).  
**Install:** see [CLI overview](./README.md#2-installation).

---

## 1. What you configure

Each JSON config file has three top-level sections. Together they define what the validator scheduler uses for that vote account:

| Section | Controls | Example |
|---------|----------|---------|
| **`block_engine`** | Block-engine endpoint URLs + bundle quotas | `https://be.example` with `max_bundles` / `period_ms` |
| **`p2c`** | Post-pack confirmation (P2C) gRPC endpoints | `https://p2c.example` — where consumers receive post-pack streams |
| **`virtual_priority`** | Account pubkey → priority multiplier | `TokenkegQ…` → `10.0` for higher scheduling priority |

- **Global** PDA: network-wide defaults (manager).
- **Validator** PDA (per `--vote`): live overrides for one vote account (manager).
- **Proposal** PDA: operator submits a draft; manager `approve` or `reject`.
- **`union`**: shows merged effective config (global + validator, by set name).

This CLI does **not** manage P2C subscription billing or tip/revshare accounts — only validator **endpoint and priority configuration**.

---

## 2. Global flags

| Flag | Default | Description |
|------|---------|-------------|
| `-k`, `--keypair` | `~/.config/solana/id.json` | Signer (manager or operator depending on command) |
| `-u`, `--url` | `t` (testnet) | RPC URL or moniker: `l`, `t`, `d`, `m` |
| `-p`, `--program-id` | `4uGNMjJFxgE3TfEiPmSpvfwYah12QZbaWWZDJqZvA9F4` | Program ID |

```sh
rakurai-validator-config \
  --url <RPC_URL> \
  --program-id <PROGRAM_ID> \
  --keypair <KEYPAIR> \
  <COMMAND>
```

---

## 3. Config JSON

See [`examples/validator_config.json`](./examples/validator_config.json) (global-style multi-set) and
[`examples/validator_config_overlay.json`](./examples/validator_config_overlay.json) (per-vote overlay).

```json
{
  "block_engine": {
    "sets": [
      {
        "name": "be-main-1",
        "url": [
          {
            "url": "https://be1.example",
            "max_bundles": 100,
            "period_ms": 100,
            "max_bundle_burst": 0
          }
        ]
      }
    ]
  },
  "p2c": {
    "sets": [
      {
        "name": "p2c-main-1",
        "url": [{ "url": "https://p2c1.example" }]
      }
    ]
  },
  "virtual_priority": {
    "sets": [
      {
        "name": "vp-main-1",
        "url": [{ "key": "<PUBKEY>", "value": 10.0 }]
      }
    ]
  }
}
```

- **Block engine:** bundle submission endpoints — `url`, `max_bundles`, `period_ms`, `max_bundle_burst` (`0` = unlimited)
- **P2C (post-pack):** confirmation stream endpoints — `url` only
- **Virtual priority:** per-account scheduling weight — `key` (pubkey) + `value` (`f64`, higher = more priority)

Omit `--config-file` on `global init` to create empty sets.

---

## 4. Union (client-side)

`union` merges **by entry `name`** (same for BE / P2C / VP):

1. Start from global `sets`
2. For each validator entry: same `name` → **replace whole entry**; new `name` → **append**
3. Nested `url` lists are **not** merged URL-by-URL — the whole named entry is replaced

| Global | Validator | Union |
|--------|-----------|--------|
| `A`, `B` | `C` | `A`, `B`, `C` |
| `A`, `B` | `A'` (same name, different URLs) | `A'`, `B` |
| `A`, `B` | empty | `A`, `B` |

---

## 5. Commands

### Global

```sh
rakurai-validator-config global init --config-file cli/examples/validator_config.json
rakurai-validator-config global update --config-file cli/examples/validator_config.json
rakurai-validator-config global show
rakurai-validator-config global close
```

### Validator (live PDA)

```sh
rakurai-validator-config validator init --vote <VOTE_PUBKEY> --operator <OPERATOR_PUBKEY>
rakurai-validator-config validator set-operator --vote <VOTE_PUBKEY> --operator <OPERATOR_PUBKEY>
rakurai-validator-config validator update --vote <VOTE_PUBKEY> \
  --config-file cli/examples/validator_config_overlay.json
rakurai-validator-config validator show --vote <VOTE_PUBKEY>
rakurai-validator-config validator close --vote <VOTE_PUBKEY>
```

### Proposal (operator draft → manager approve)

```sh
# Operator keypair
rakurai-validator-config -k operator.json proposal submit --vote <VOTE> \
  --config-file cli/examples/validator_config_overlay.json
rakurai-validator-config proposal show --vote <VOTE>

# Manager keypair
rakurai-validator-config -k manager.json proposal approve --vote <VOTE>
rakurai-validator-config -k manager.json proposal reject --vote <VOTE>
```

### Union

```sh
rakurai-validator-config union --vote <VOTE_PUBKEY>
```

---

## 6. Proposal flow

1. Manager `validator init --operator <OP>`
2. Operator `proposal submit` (creates/updates draft PDA; live config unchanged)
3. Manager `proposal approve` (draft → live validator) or `proposal reject`

---

## 7. Help

```sh
rakurai-validator-config --help
rakurai-validator-config global --help
rakurai-validator-config validator --help
rakurai-validator-config proposal --help
```
