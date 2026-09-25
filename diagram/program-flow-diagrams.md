# Program Procedure Flow Diagrams

High-level procedure flows for Rakurai on-chain programs. Production is **V1 only**. Reward Distribution has **four revenue models**: RCA, TCA, MCA, PSA.

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

## 2. Reward Distribution — RCA, TCA, MCA, PSA

Per-epoch **RCA** for block rewards (Merkle staker claims). Independent money paths: **TCA** (tips), **PSA** (prepaid P2C subscription), **MCA** (post-pack backrun share).

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

    subgraph tcaMca [TCA / MCA — RevenueShareAccountV1]
        P0[initialize_revenue_share_account_v1]
        P0 -->|Tip = TCA / MevShare = MCA| PTS[REVENUE_SHARE_V1 PDA]
        P1[record_revenue_v1]
        P1 -->|TCA each leader turn; MCA once post-epoch| LEDGER[Epoch ledger]
        P1b[settle_revenue — partner custom TCA / MCA]
        P1b --> LEDGER
        P2[claim_revenue_v1]
        P2 -->|pays transferred_amount; 0 bps for rakurai TCA| PAY[commission_account + validator identity]
    end

    subgraph psa [PSA — P2CSubscriptionAccount]
        S0[initialize_p2c_subscription_account]
        S1[fund_p2c_subscription]
        S2[record_p2c_subscription]
        S3[claim_epoch_p2c_subscription]
        S0 --> S1 --> S2 --> S3
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
| Tips/Mev config | Setup | `initialize_tips_and_mev_share_config` / `update_*` |
| TCA / MCA | Setup | `initialize_revenue_share_account_v1` |
| TCA / MCA | Record / settle / claim | `record_revenue_v1`, `settle_revenue`, `claim_revenue_v1` |
| PSA | Fund / record / claim | `fund_p2c_subscription`, `record_p2c_subscription`, `claim_epoch_p2c_subscription` |

**TCA / MCA PDA:** `[REVENUE_SHARE_V1, TIP|MEV_SHARE, name, vote]`

**PSA PDA:** `[P2C_SUBSCRIPTION, name, vote]`

---

## 3. Rakurai Tip Manager

Global **8 tip PDAs** + singleton config. Validators drain on leader turns into the Rakurai **TCA**.

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
        PREV[change_tip_receiver_v2 — new receiver must be TCA]
        PREV --> DRAIN[Drain 8 tip PDAs]
        DRAIN --> CFG2[config.validator_tip_receiver = new TCA]
        PREV -->|CPI| RECV[record_revenue_v1]
    end

    subgraph admin [Admin]
        A1[change_client]
        A2[close_rakurai_tip_manager]
    end

    setup --> users
    users --> drain
```

| Step | Instruction | Who |
|------|-------------|-----|
| Deploy | `initialize_rakurai_tip_manager` | payer (once) |
| Drain | `change_tip_receiver_v2` | Rakurai-enabled validator |
| Rotate client | `change_client` | config authority |
| Shutdown | `close_rakurai_tip_manager` | config authority |

**Corner cases:** Drain credits `old_tip_receiver`, not `new_tip_receiver`. First drain after tip-manager init credits init payer until config already points at a TCA.

---

## 4. Client Config

Scheduler **endpoints and virtual-priority tip maps** only (not money). Every write **replaces the entire `Config`** — submit **current + new**.

```mermaid
flowchart TD
    G[init_global / update_global — full Config]
    V[init_validator copies global]
    V --> VU[update_validator — full Config]
    P[operator: init_proposal / update_proposal — full Config]
    P --> A[manager: approve_proposal copies draft to live validator]
    U[union — read-only: validator PDA if present, else global]
```

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
        TM->>RD: change_tip_receiver_v2 then CPI record_revenue_v1
        Note over TM: drain to TCA
    end
    Note over RD: Post-epoch
    RD->>RD: upload_merkle_root and claim
    RD->>RD: claim_revenue_v1 rakurai 0 bps already taken on tip drain
```
