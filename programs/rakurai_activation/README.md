# Rakurai Activation Program

A multisig-based Solana smart contract for enabling or disabling the Rakurai scheduler. It also governs the commission on block rewards for both the block builder (i.e., Rakurai) and the validator.

Note: The remaining block rewards after deducting the two commissions are distributed to stakers via the [`RewardDistributionProgram`](../reward_distribution/README.md).

➤ For more details, refer to the [IDL File](./idl/rakurai_activation.json).

---

## Purpose

Each validator must create a **Rakurai Activation Account (RAA)** — a **PDA jointly controlled by both the validator and Rakurai**.

This account governs:
- Whether the validator is **actively using the Rakurai Scheduler** to schedule their blocks or not.
- The **commission percentage** the validator wants to retain from total block rewards.
- Rakurai's commission from total block rewards (set during initialization, read from global config (**Rakurai Activation Config Account**)).

---

## Multisig Control

This program implements a 2-party asynchronous multisig:

- **Enabling Rakurai Scheduler** → Requires **2/2 multisig approval**  
  One party (Validator or Rakurai) proposes, the other approves.
  
- **Disabling Scheduler** → Can be done **unilaterally (1/2 multisig)**  
  Either party can act independently to disable.

> Unlike traditional multisig, both parties do not sign the same transaction. Instead, actions are proposed and approved via separate transactions.
---

## RakuraiActivationAccount Account Creation

- The validator initializes their **RakuraiActivationAccount** PDA using:
  - Their **identity pubkey**.
  - A seed constant.
- During creation, the validator specifies:
  - `validator_commission_bps` (0–10000) — validator wants to retain from total block rewards.
  - Rakurai's commission is fetched from a global config account (**Rakurai Activation Config Account**), a PDA under the same program.

Once created, this account:
- Authorizes Rakurai reward logic on-chain.
- Enables the validator to use Rakurai's scheduler for enhanced performance and MEV rewards.

---

## Commission Updates

- The validator may update their [**commission percentage**](../../cli/README.md#3-update-commission) at any time.
- The updated commission applies either:
  - From the **current epoch**, if no [`RewardCollectionAccount`](../reward_distribution/README.md#1-rewardcollectionaccount-account-initialization) has been initialized yet.
  - Or from the **next epoch**, if already initialized.

---

## Activation Flow

1. **Enabling Rakurai**:
   - The validator submits a [`update_rakurai_activation_approval`](../../cli/README.md#2-scheduler-control) transaction.
   - In response to that transaction, Rakurai submits a transaction to approve and activate the Rakurai scheduler.

2. **Disabling Rakurai**:
   - Either party (Rakurai or Validator) can unilaterally disable the Rakurai scheduler.

3. **Re-enabling**:
   - Requires both validator and Rakurai to propose -> approve via new transactions.

> Activation status is respected in reward distribution and scheduling logic across Rakurai-integrated programs.

---

## [CLI Tool](../../cli/README.md)

A CLI tool is available for operators to:
- Initialize their RakuraiActivationAccount.
- Update commission settings.
- Enable/disable the Rakurai scheduler.

---
