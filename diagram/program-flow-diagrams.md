# Program Procedure Flow Diagrams

High-level procedure flows for the three Rakurai on-chain programs on `feature/tip_distribution`.

---

## 1. Rakurai Activation

One **config PDA** (global) and one **RAA PDA** per validator identity. Multisig-style enable/disable for the Rakurai scheduler.

```mermaid
flowchart TD
    subgraph setup [One-time setup]
        A1[Authority: initialize config PDA]
    end

    subgraph perValidator [Per validator]
        V1[Validator: initialize_rakurai_activation_account]
        V1 --> RAA[RakuraiActivationAccount PDA]
        RAA --> V2[Set validator_commission_bps]
    end

    subgraph enable [Enable scheduler — 2/2]
        E1[Party A: update_rakurai_activation_approval propose enable]
        E1 --> E2[Party B: update_rakurai_activation_approval approve]
        E2 --> ON[is_enabled = true]
    end

    subgraph disable [Disable scheduler — 1/2]
        D1[Validator OR Rakurai: update_rakurai_activation_approval disable]
        D1 --> OFF[is_enabled = false]
    end

    subgraph ongoing [Ongoing]
        C1[Validator: update_rakurai_activation_commission]
        X1[Block builder: close_rakurai_activation_account]
    end

    setup --> perValidator
    RAA --> enable
    RAA --> disable
    ON --> RD[Used by reward_distribution RCA init + tip_manager drain]
```

| Step | Instruction | Who |
|------|-------------|-----|
| Global config | `initialize` / `update_config` | config authority |
| Create RAA | `initialize_rakurai_activation_account` | validator identity |
| Enable | `update_rakurai_activation_approval` (propose → approve) | validator + Rakurai |
| Disable | `update_rakurai_activation_approval` | validator **or** Rakurai |
| Commission | `update_rakurai_activation_commission` | validator |
| Close RAA | `close_rakurai_activation_account` | block builder authority |

---

## 2. Reward Distribution — RCA & Revenue-Share Accounts

Per-epoch **RCA** for block rewards (Merkle staker claims) and per-validator **revenue-share vaults** for tip/backrun attribution.

```mermaid
flowchart TD
    subgraph config [Config]
        RD0[initialize / update_config / close_config]
    end

    subgraph rca [RCA — block rewards per vote + epoch]
        R1[Epoch start: initialize_reward_collection_account]
        R1 -->|requires enabled RAA + vote auth| RCA[RewardCollectionAccount]
        R2[Each leader turn: transfer_staker_rewards]
        R2 --> RCA
        R3[Post-epoch: upload_merkle_root + staker claim + close_claim_status]
        R3 --> RCA
        R4[After expiry: close_reward_collection_account]
    end

    subgraph revenue [Revenue-share vaults per share_kind + label + vote]
        P0[Rakurai: initialize_revenue_share_account share_kind = Tip or Backrun]
        P0 --> PTS[RevenueShareAccount share_kind in PDA seeds]
        P1[Leader turn: record_revenue]
        P1 -->|accounting only| LEDGER[Epoch ledger + convert_to_block_rewards snapshot]
        P2[Post-epoch: claim_revenue]
        P2 -->|commission_bps split| PAY[commission_account + validator identity]
        P3[Manager: update_revenue_share_config / close]
    end

    config --> rca
    config --> revenue
```

| Path | Phase | Instructions |
|------|-------|--------------|
| RCA | Epoch start | `initialize_reward_collection_account` |
| RCA | Leader turns | `transfer_staker_rewards`, optional MEV commission ix |
| RCA | Post-epoch | `upload_merkle_root`, `claim`, `close_claim_status` |
| RCA | Cleanup | `close_reward_collection_account` |
| Revenue | Setup | `initialize_revenue_share_account` (`share_kind` arg) |
| Revenue | Leader turns | `record_revenue` |
| Revenue | Post-epoch | `claim_revenue` |
| Revenue | Admin | `update_revenue_share_config`, `close_revenue_share_account` |

Revenue-share PDA seeds: `[REVENUE_SHARE, share_kind ("TIP" \| "BACKRUN"), name, validator_vote]`. One unified `RevenueShareAccount` (aliases `TipsCollectionAccount` (TCA) / `BackrunCollectionAccount` (BCA)); `share_kind` selects Tip vs Backrun. Claim requires revenue-share PDA lamports ≥ ledger amount.

---

## 3. Rakurai Tip Manager

Global **8 tip PDAs** + singleton config. Validators drain on leader turns; config receiver rotates to tip revenue-share PDA.

```mermaid
flowchart TD
    subgraph setup [One-time setup]
        T0[initialize_rakurai_tip_manager]
        T0 --> CFG[TipManagerConfigAccount]
        T0 --> TIPS[8 × RakuraiTipAccount PDAs]
    end

    subgraph users [Tip intake]
        U1[Users send SOL to any tip PDA]
        U1 --> TIPS
    end

    subgraph drain [Validator leader turn]
        PRE[Prerequisite: revenue-share PDA Tip kind initialized on reward_distribution]
        V1[change_tip_receiver]
        V1 -->|RAA enabled + vote auth| DRAIN[Drain 8 tip PDAs]
        DRAIN --> SPLIT[Split by block_builder_commission_bps]
        SPLIT --> OLD[validator_fee → old_tip_receiver]
        SPLIT --> BB[block_builder_fee → commission account]
        V1 --> CFG2[config.validator_tip_receiver = tip revenue-share PDA]
    end

    subgraph admin [Admin]
        A1[change_block_builder]
        A2[close_rakurai_tip_manager]
    end

    setup --> users
    users --> drain
    PRE --> V1
```

| Step | Instruction | Who |
|------|-------------|-----|
| Deploy | `initialize_rakurai_tip_manager` | payer (once) |
| Drain + rotate | `change_tip_receiver` | Rakurai-enabled validator |
| Rotate builder | `change_block_builder` | config authority |
| Shutdown | `close_rakurai_tip_manager` | config authority |

**Corner cases:** Current drain credits `old_tip_receiver`, not `new_tip_receiver`. First drain after tip-manager init credits init payer until config already points at the revenue-share PDA. Revenue-share vault must exist before `change_tip_receiver` succeeds.

---

## Cross-program sequence (validator epoch)

```mermaid
sequenceDiagram
    participant Act as rakurai_activation
    participant RD as reward_distribution
    participant TM as rakurai_tip_manager

    Note over Act: RAA enabled (2/2)
    RD->>RD: initialize_reward_collection_account
    RD->>RD: initialize_revenue_share_account (Tip, Rakurai)
    loop Leader turns
        RD->>RD: transfer_staker_rewards
        RD->>RD: record_revenue
        TM->>TM: change_tip_receiver → revenue-share PDA
    end
    Note over RD: Post-epoch
    RD->>RD: upload_merkle_root + claim
    RD->>RD: claim_revenue
```