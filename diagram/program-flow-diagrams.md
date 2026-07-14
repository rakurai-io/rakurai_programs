# Program Procedure Flow Diagrams

High-level procedure flows for the three Rakurai on-chain programs.

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
        X1[Client: close_rakurai_activation_account]
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
| Close RAA | `close_rakurai_activation_account` | client authority |

---

## 2. Reward Distribution — RCA & Revenue-Share Accounts

Per-epoch **RCA** for block rewards (Merkle staker claims) and per-validator **revenue-share vaults** for tip/mev-share attribution.

```mermaid
flowchart TD
    subgraph config [Config]
        RD0[initialize / update_config / close_config]
        TM0[initialize_tips_and_mev_share_config]
        TM0 --> TMC[TipsAndMevShareConfigAccount]
    end

    subgraph rca [RCA — block rewards per vote + epoch]
        R1[Epoch start: initialize_reward_collection_account_v1]
        R1 -->|requires enabled RAA + vote auth| RCA[RewardCollectionAccount]
        R2[Each leader turn: transfer_staker_rewards]
        R2 --> RCA
        R3[Post-epoch: upload_merkle_root + staker claim + close_claim_status]
        R3 --> RCA
        R4[After expiry: close_reward_collection_account]
    end

    subgraph revenue [Revenue-share vaults per share_kind + label + vote]
        P0[initialize_revenue_share_account_v1]
        P0 -->|defaults from TipsAndMevShareConfig by share_kind| PTS[RevenueShareAccount Tip or MevShare]
        P1[Leader turn: record_revenue]
        P1 -->|accounting only| LEDGER[Epoch ledger]
        P2[Post-epoch: claim_revenue]
        P2 -->|bps; 0 for rakurai vault avoid double fee| PAY[commission_account + validator identity]
        P3[Manager: update_revenue_share_config / close]
    end

    config --> rca
    TMC --> P0
```

| Path | Phase | Instructions |
|------|-------|--------------|
| RCA | Epoch start | `initialize_reward_collection_account_v1` |
| RCA | Leader turns | `transfer_staker_rewards`, optional MEV commission ix |
| RCA | Post-epoch | `upload_merkle_root`, `claim`, `close_claim_status` |
| RCA | Cleanup | `close_reward_collection_account` |
| Tips/Mev config | Setup | `initialize_tips_and_mev_share_config` / `update_tips_and_mev_share_config` |
| Revenue | Setup | `initialize_revenue_share_account_v1` |
| Revenue | Leader turns | `record_revenue` |
| Revenue | Post-epoch | `claim_revenue` (Rakurai name skips commission) |
| Revenue | Admin | `update_revenue_share_config`, `close_revenue_share_account` |

Revenue-share PDA seeds: `[REVENUE_SHARE, share_kind ("TIP" \| "MEV_SHARE"), name, validator_vote]`. One unified `RevenueShareAccount` (aliases `TipsCollectionAccount` (TCA) / `MevShareCollectionAccount` (MCA)). Claim requires revenue-share PDA lamports ≥ ledger amount. For `name == RAKURAI_REVENUE_NAME`, claim commission is 0 because tip-manager drain already took Rakurai’s cut; partner vaults still apply `commission_bps` at claim.

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
        PRE[Prerequisite: Rakurai TCA initialized on reward_distribution]
        V2[change_tip_receiver_v2]
        V2 -->|RAA enabled| DRAIN[Drain 8 tip PDAs]
        DRAIN --> SPLIT[Split by old TCA commission_bps]
        SPLIT --> OLD[validator_fee → old TCA]
        SPLIT --> BB[client_fee → old TCA commission_account]
        V2 -->|optional CPI| REC[record_revenue on old TCA]
        V2 --> CFG2[config.validator_tip_receiver = new TCA]
    end

    subgraph admin [Admin]
        A1[change_client]
        A2[close_rakurai_tip_manager]
    end

    setup --> users
    users --> drain
    PRE --> V2
```

| Step | Instruction | Who |
|------|-------------|-----|
| Deploy | `initialize_rakurai_tip_manager` | payer (once) |
| Drain + rotate | `change_tip_receiver_v2` | Rakurai-enabled validator |
| Rotate client | `change_client` | config authority |
| Shutdown | `close_rakurai_tip_manager` | config authority |

**Corner cases:** Drain credits `old_tip_receiver`, not `new_tip_receiver`. First drain after tip-manager init credits init payer until config already points at a TCA. Both old and new receivers must be TCAs.

---

## Cross-program sequence (validator epoch)

```mermaid
sequenceDiagram
    participant Act as rakurai_activation
    participant RD as reward_distribution
    participant TM as rakurai_tip_manager

    Note over Act: RAA enabled (2 of 2)
    RD->>RD: initialize_tips_and_mev_share_config (once)
    RD->>RD: initialize_reward_collection_account_v1
    RD->>RD: initialize_revenue_share_account_v1 Tip rakurai
    loop Leader turns
        RD->>RD: transfer_staker_rewards
        TM->>RD: change_tip_receiver_v2 then CPI record_revenue
        Note over TM: drain and old TCA commission then config to TCA
    end
    Note over RD: Post-epoch
    RD->>RD: upload_merkle_root and claim
    RD->>RD: claim_revenue rakurai 0 bps fee already on tip drain
```
