#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use rakurai_activation::state::RakuraiActivationAccount;

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::RakuraiTipManagerError::{ArithmeticError, Unauthorized};
use reward_distribution::state::{TipsCollectionAccount, TipsCollectionAccountV1};

/// Rakurai label for tip revenue share vaults; defined once in reward_distribution.
pub use reward_distribution::state::RAKURAI_REVENUE_NAME;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Rakurai Tip Manager Program",
    project_url: "https://rakurai.io/",
    contacts: "link:https://rakurai.io/company,link:https://discord.gg/QzqQVBAMpp,link:https://t.me/rakurai_official,link:https://github.com/rakurai-io/rakurai-validator,link:https://docs.rakurai.io",
    policy: "https://rakurai.io/faqs",
    // Optional fields
    preferred_languages: "en",
    source_code: "https://github.com/rakurai-io/rakurai_programs"
}

pub mod sdk;

declare_id!("4qRZaFzf7MvgfBTCP9grb69cCST8UmKHPtkpGAgkJosD");

/// PDA Seeds

/// Seed for the singleton configuration account
pub const TIP_MANAGER_CONFIG_ACCOUNT_SEED: &[u8] = b"TIP_MANAGER_CONFIG_ACCOUNT";
/// Seeds for Rakurai tip accounts
pub const RAKURAI_TIP_ACCOUNT_0_SEED: &[u8] = b"RAKURAI_TIP_ACCOUNT_0";
pub const RAKURAI_TIP_ACCOUNT_1_SEED: &[u8] = b"RAKURAI_TIP_ACCOUNT_1";
pub const RAKURAI_TIP_ACCOUNT_2_SEED: &[u8] = b"RAKURAI_TIP_ACCOUNT_2";
pub const RAKURAI_TIP_ACCOUNT_3_SEED: &[u8] = b"RAKURAI_TIP_ACCOUNT_3";
pub const RAKURAI_TIP_ACCOUNT_4_SEED: &[u8] = b"RAKURAI_TIP_ACCOUNT_4";
pub const RAKURAI_TIP_ACCOUNT_5_SEED: &[u8] = b"RAKURAI_TIP_ACCOUNT_5";
pub const RAKURAI_TIP_ACCOUNT_6_SEED: &[u8] = b"RAKURAI_TIP_ACCOUNT_6";
pub const RAKURAI_TIP_ACCOUNT_7_SEED: &[u8] = b"RAKURAI_TIP_ACCOUNT_7";
/// Seed for the PDA that signs `reward_distribution::record_revenue` CPIs. Set this PDA as a
/// `TipsCollectionAccount` (TCA) `record_authority` so the tip manager can record drained tips
/// against the receiving TCA.
pub const RECORD_AUTHORITY_SEED: &[u8] = b"RECORD_AUTHORITY";

const MAX_COMMISSION_BPS: u64 = 10_000;
/// Rakurai Tip Manager Program: users send tips to one of eight tip accounts, validators periodically drain them
/// and tips are split between the configured tip receiver and an client commission account.
#[program]
pub mod rakurai_tip_manager {
    use super::*;

    /// Initializes the Rakurai Tip Manager by creating the singleton config PDA and eight Rakurai tip PDAs,
    /// this instruction must be executed exactly once.
    pub fn initialize_rakurai_tip_manager(
        ctx: Context<InitializeRakuraiTipManager>,
        bumps: RakuraiTipManagerBumps,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.tip_manager_config;

        cfg.validator_tip_receiver_account = ctx.accounts.payer.key();
        cfg.client_commission_account = ctx.accounts.payer.key();
        cfg.client_commission_bps = MAX_COMMISSION_BPS;
        cfg.authority = ctx.accounts.payer.key();

        cfg.bumps = bumps;

        Ok(())
    }

    /// Closes all Tip manager program accounts (8 tip accounts + config account) and reclaim rent.
    /// Only the tip manager config authority can invoke this instruction.

    pub fn close_rakurai_tip_manager(ctx: Context<CloseRakuraiTipManager>) -> Result<()> {
        // Verify caller authority
        CloseRakuraiTipManager::auth(&ctx)?;

        let authority = &mut ctx.accounts.signer;
        let mut lamports_reclaimed = 0;

        let config_account = &mut ctx.accounts.tip_manager_config;
        // Transfer all lamports from config to authority
        let lamports_to_reclaim = config_account.to_account_info().lamports();
        **config_account.to_account_info().try_borrow_mut_lamports()? = 0;
        **authority.try_borrow_mut_lamports()? = authority
            .lamports()
            .checked_add(lamports_to_reclaim)
            .ok_or(ArithmeticError)?;
        lamports_reclaimed += lamports_to_reclaim;

        // Close All Tip Accounts
        lamports_reclaimed += RakuraiTipAccount::close_account(
            authority,
            &ctx.accounts.rakurai_tip_account_0.to_account_info(),
        )?;
        lamports_reclaimed += RakuraiTipAccount::close_account(
            authority,
            &ctx.accounts.rakurai_tip_account_1.to_account_info(),
        )?;
        lamports_reclaimed += RakuraiTipAccount::close_account(
            authority,
            &ctx.accounts.rakurai_tip_account_2.to_account_info(),
        )?;
        lamports_reclaimed += RakuraiTipAccount::close_account(
            authority,
            &ctx.accounts.rakurai_tip_account_3.to_account_info(),
        )?;
        lamports_reclaimed += RakuraiTipAccount::close_account(
            authority,
            &ctx.accounts.rakurai_tip_account_4.to_account_info(),
        )?;
        lamports_reclaimed += RakuraiTipAccount::close_account(
            authority,
            &ctx.accounts.rakurai_tip_account_5.to_account_info(),
        )?;
        lamports_reclaimed += RakuraiTipAccount::close_account(
            authority,
            &ctx.accounts.rakurai_tip_account_6.to_account_info(),
        )?;
        lamports_reclaimed += RakuraiTipAccount::close_account(
            authority,
            &ctx.accounts.rakurai_tip_account_7.to_account_info(),
        )?;

        emit!(TipsManagerCloseEvent {
            close_authority: authority.key(),
            lamports_reclaimed,
        });
        Ok(())
    }

    /// Changes the active tip receiver (legacy). Drains tips and rotates config receiver.
    /// Prefer `change_tip_receiver_v1` for RAA gate, vote auth, and TCA validation.
    pub fn change_tip_receiver(ctx: Context<ChangeTipReceiver>) -> Result<()> {
        let rent = Rent::get()?;
        let tip_accounts = ctx.accounts.get_tip_accounts();

        let total_tips = RakuraiTipAccount::drain_accounts(&rent, &tip_accounts)?;

        let client_fee = total_tips
            .checked_mul(ctx.accounts.tip_manager_config.client_commission_bps)
            .ok_or(ArithmeticError)?
            .checked_div(MAX_COMMISSION_BPS)
            .ok_or(ArithmeticError)?;

        let validator_fee = total_tips.checked_sub(client_fee).ok_or(ArithmeticError)?;

        if validator_fee > 0 {
            **ctx.accounts.old_tip_receiver.try_borrow_mut_lamports()? += validator_fee;
        }

        if client_fee > 0 {
            **ctx
                .accounts
                .client_commission_account
                .try_borrow_mut_lamports()? += client_fee;
        }

        if client_fee > 0 || validator_fee > 0 {
            emit!(TipsClaimedEvent {
                validator_tip_receiver_account: ctx.accounts.old_tip_receiver.key(),
                tip_receiver_amount: validator_fee,
                client_commission_account: ctx.accounts.client_commission_account.key(),
                client_amount: client_fee,
            });
        }

        ctx.accounts
            .tip_manager_config
            .validator_tip_receiver_account = ctx.accounts.new_tip_receiver.key();

        Ok(())
    }

    /// Changes the active tip receiver and drains tips to `old_tip_receiver` (wallet or TCA).
    /// Commission on the drain uses tip-manager **global** config (set by the previous leader from
    /// their TCA). After the drain, global commission is synced from the **new** TCA for the next
    /// leader. When `old_tip_receiver` is a TCA, CPIs `record_revenue` to update the ledger.
    pub fn change_tip_receiver_v1(ctx: Context<ChangeTipReceiverV1>) -> Result<()> {
        ChangeTipReceiverV1::auth(&ctx)?;

        let rent = Rent::get()?;
        let tip_accounts = ctx.accounts.get_tip_accounts();

        // 1. Collect per-account tips and total WITHOUT draining yet.
        let (total_tips, per_account_tips) = RakuraiTipAccount::collect_tips(&rent, &tip_accounts)?;

        let client_fee = total_tips
            .checked_mul(ctx.accounts.tip_manager_config.client_commission_bps)
            .ok_or(ArithmeticError)?
            .checked_div(MAX_COMMISSION_BPS)
            .ok_or(ArithmeticError)?;

        let validator_fee = total_tips.checked_sub(client_fee).ok_or(ArithmeticError)?;

        // 2. Record on old TCA or TCAV1 when possible (mixed v1↔v2 handoffs).
        // CPI before lamport changes (runtime balance sync).
        maybe_record_tip_revenue(
            &ctx.accounts.old_tip_receiver.to_account_info(),
            &ctx.accounts.record_authority.to_account_info(),
            ctx.bumps.record_authority,
            ctx.remaining_accounts[1].key(),
            validator_fee,
        )?;

        // 3. Drain using precomputed per-account tips, then distribute lamports.
        RakuraiTipAccount::drain_collected(&tip_accounts, &per_account_tips)?;

        if validator_fee > 0 {
            **ctx.accounts.old_tip_receiver.try_borrow_mut_lamports()? += validator_fee;
        }
        if client_fee > 0 {
            **ctx
                .accounts
                .client_commission_account
                .try_borrow_mut_lamports()? += client_fee;
        }

        if client_fee > 0 || validator_fee > 0 {
            emit!(TipsClaimedEvent {
                validator_tip_receiver_account: ctx.accounts.old_tip_receiver.key(),
                tip_receiver_amount: validator_fee,
                client_commission_account: ctx.accounts.client_commission_account.key(),
                client_amount: client_fee,
            });
        }

        let new_tip_receiver = ctx.accounts.new_tip_receiver.to_account_info();
        ctx.accounts
            .tip_manager_config
            .validator_tip_receiver_account = new_tip_receiver.key();
        ctx.accounts.tip_manager_config.client_commission_bps =
            ctx.accounts.new_tip_receiver.commission_bps as u64;
        Ok(())
    }

    /// Mirror of `change_tip_receiver_v1` for **TCAV1** (`REVENUE_SHARE_V1`).
    /// Commission on the drain uses tip-manager global config; after drain, syncs global from the
    /// **new** TCAV1. Records against legacy TCA or TCAV1 old receiver.
    pub fn change_tip_receiver_v2(ctx: Context<ChangeTipReceiverV2>) -> Result<()> {
        ChangeTipReceiverV2::auth(&ctx)?;

        let rent = Rent::get()?;
        let tip_accounts = ctx.accounts.get_tip_accounts();

        let (total_tips, per_account_tips) = RakuraiTipAccount::collect_tips(&rent, &tip_accounts)?;

        let client_fee = total_tips
            .checked_mul(ctx.accounts.tip_manager_config.client_commission_bps)
            .ok_or(ArithmeticError)?
            .checked_div(MAX_COMMISSION_BPS)
            .ok_or(ArithmeticError)?;

        let validator_fee = total_tips.checked_sub(client_fee).ok_or(ArithmeticError)?;

        maybe_record_tip_revenue(
            &ctx.accounts.old_tip_receiver.to_account_info(),
            &ctx.accounts.record_authority.to_account_info(),
            ctx.bumps.record_authority,
            ctx.remaining_accounts[1].key(),
            validator_fee,
        )?;

        RakuraiTipAccount::drain_collected(&tip_accounts, &per_account_tips)?;

        if validator_fee > 0 {
            **ctx.accounts.old_tip_receiver.try_borrow_mut_lamports()? += validator_fee;
        }
        if client_fee > 0 {
            **ctx
                .accounts
                .client_commission_account
                .try_borrow_mut_lamports()? += client_fee;
        }

        if client_fee > 0 || validator_fee > 0 {
            emit!(TipsClaimedEvent {
                validator_tip_receiver_account: ctx.accounts.old_tip_receiver.key(),
                tip_receiver_amount: validator_fee,
                client_commission_account: ctx.accounts.client_commission_account.key(),
                client_amount: client_fee,
            });
        }

        let new_tip_receiver = ctx.accounts.new_tip_receiver.to_account_info();
        ctx.accounts
            .tip_manager_config
            .validator_tip_receiver_account = new_tip_receiver.key();
        ctx.accounts.tip_manager_config.client_commission_bps =
            ctx.accounts.new_tip_receiver.commission_bps as u64;

        Ok(())
    }

    /// Changes the client and its commission by first draining all pending tips (distributing shares to the tip receiver
    /// and old client) and then setting the new client and its commission.
    pub fn change_client(ctx: Context<ChangeClient>, client_commission_bps: u64) -> Result<()> {
        ChangeClient::auth(&ctx)?;
        require_gte!(
            MAX_COMMISSION_BPS,
            client_commission_bps,
            RakuraiTipManagerError::MaxCommissionBpsExceeded
        );
        let rent = Rent::get()?;
        let total_tips =
            RakuraiTipAccount::drain_accounts(&rent, &ctx.accounts.get_tip_accounts())?;

        let client_fee = total_tips
            .checked_mul(ctx.accounts.tip_manager_config.client_commission_bps)
            .ok_or(ArithmeticError)?
            .checked_div(MAX_COMMISSION_BPS)
            .ok_or(ArithmeticError)?;

        let validator_fee = total_tips.checked_sub(client_fee).ok_or(ArithmeticError)?;

        if validator_fee > 0 {
            **ctx
                .accounts
                .validator_tip_receiver_account
                .try_borrow_mut_lamports()? += validator_fee;
        }

        if client_fee > 0 {
            **ctx.accounts.old_client.try_borrow_mut_lamports()? += client_fee;
        }

        if client_fee > 0 || validator_fee > 0 {
            emit!(TipsClaimedEvent {
                validator_tip_receiver_account: ctx.accounts.validator_tip_receiver_account.key(),
                tip_receiver_amount: validator_fee,
                client_commission_account: ctx.accounts.old_client.key(),
                client_amount: client_fee,
            });
        }

        ctx.accounts.tip_manager_config.client_commission_account = ctx.accounts.new_client.key();
        ctx.accounts.tip_manager_config.client_commission_bps = client_commission_bps;

        Ok(())
    }
}

/// CPI `record_revenue` (legacy TCA) or `record_revenue_v1` (TCAV1) when `old_tip_receiver`
/// is a matching vault. Skips recording (does not error) for wallets / wrong authority / unknown layout.
///
/// Used by both `change_tip_receiver_v1` and `change_tip_receiver_v2` so mixed handoffs still ledger.
fn maybe_record_tip_revenue<'info>(
    old_tip_receiver: &AccountInfo<'info>,
    record_authority: &AccountInfo<'info>,
    record_authority_bump: u8,
    reward_distribution_program: Pubkey,
    amount: u64,
) -> Result<()> {
    if amount == 0 || old_tip_receiver.owner != &reward_distribution::ID {
        return Ok(());
    }

    use anchor_lang::solana_program::program::invoke_signed;
    use anchor_lang::AccountDeserialize;
    use reward_distribution::sdk::instruction::{
        record_revenue_ix, record_revenue_v1_ix, RecordRevenueArgs, RecordRevenueShareAccounts,
    };

    let record_accounts = RecordRevenueShareAccounts {
        revenue_share_account: old_tip_receiver.key(),
        record_authority: record_authority.key(),
    };
    let args = RecordRevenueArgs { amount };
    let signer_seeds: &[&[&[u8]]] = &[&[RECORD_AUTHORITY_SEED, &[record_authority_bump]]];

    {
        let data = old_tip_receiver.data.borrow();
        if let Ok(tca) = TipsCollectionAccount::try_deserialize(&mut &data[..]) {
            if tca.record_authority != record_authority.key() {
                return Ok(());
            }
            drop(data);
            let record_ix = record_revenue_ix(reward_distribution_program, args, record_accounts);
            invoke_signed(
                &record_ix,
                &[old_tip_receiver.clone(), record_authority.clone()],
                signer_seeds,
            )?;
            return Ok(());
        }
    }

    {
        let data = old_tip_receiver.data.borrow();
        if let Ok(tca) = TipsCollectionAccountV1::try_deserialize(&mut &data[..]) {
            if tca.record_authority != record_authority.key() {
                return Ok(());
            }
            drop(data);
            let record_ix =
                record_revenue_v1_ix(reward_distribution_program, args, record_accounts);
            invoke_signed(
                &record_ix,
                &[old_tip_receiver.clone(), record_authority.clone()],
                signer_seeds,
            )?;
        }
    }

    Ok(())
}

/// Errors
#[error_code]
pub enum RakuraiTipManagerError {
    #[msg("Encountered an arithmetic under/overflow error.")]
    ArithmeticError,

    #[msg("Client commission basis points must be less than or equal to 10_000")]
    MaxCommissionBpsExceeded,

    #[msg("Unauthorized signer.")]
    Unauthorized,

    #[msg("Rakurai scheduler is not enabled for this validator.")]
    RakuraiSchedulerNotEnabled,
}

/// PDA Bumps
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct RakuraiTipManagerBumps {
    pub tip_manager_config: u8,
    pub rakurai_tip_account_0: u8,
    pub rakurai_tip_account_1: u8,
    pub rakurai_tip_account_2: u8,
    pub rakurai_tip_account_3: u8,
    pub rakurai_tip_account_4: u8,
    pub rakurai_tip_account_5: u8,
    pub rakurai_tip_account_6: u8,
    pub rakurai_tip_account_7: u8,
}

impl RakuraiTipManagerBumps {
    pub const SIZE: usize = 9;
}

#[derive(Accounts)]
#[instruction(tip_manager_bumps: RakuraiTipManagerBumps)]
pub struct InitializeRakuraiTipManager<'info> {
    /// singleton account
    #[account(
        init,
        seeds = [TIP_MANAGER_CONFIG_ACCOUNT_SEED],
        bump,
        payer = payer,
        space = TipManagerConfigAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_manager_config: Account<'info, TipManagerConfigAccount>,
    #[account(
        init,
        seeds = [RAKURAI_TIP_ACCOUNT_0_SEED],
        bump,
        payer = payer,
        space = RakuraiTipAccount::SIZE,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_0: Account<'info, RakuraiTipAccount>,
    #[account(
        init,
        seeds = [RAKURAI_TIP_ACCOUNT_1_SEED],
        bump,
        payer = payer,
        space = RakuraiTipAccount::SIZE,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_1: Account<'info, RakuraiTipAccount>,
    #[account(
        init,
        seeds = [RAKURAI_TIP_ACCOUNT_2_SEED],
        bump,
        payer = payer,
        space = RakuraiTipAccount::SIZE,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_2: Account<'info, RakuraiTipAccount>,
    #[account(
        init,
        seeds = [RAKURAI_TIP_ACCOUNT_3_SEED],
        bump,
        payer = payer,
        space = RakuraiTipAccount::SIZE,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_3: Account<'info, RakuraiTipAccount>,
    #[account(
        init,
        seeds = [RAKURAI_TIP_ACCOUNT_4_SEED],
        bump,
        payer = payer,
        space = RakuraiTipAccount::SIZE,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_4: Account<'info, RakuraiTipAccount>,
    #[account(
        init,
        seeds = [RAKURAI_TIP_ACCOUNT_5_SEED],
        bump,
        payer = payer,
        space = RakuraiTipAccount::SIZE,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_5: Account<'info, RakuraiTipAccount>,
    #[account(
        init,
        seeds = [RAKURAI_TIP_ACCOUNT_6_SEED],
        bump,
        payer = payer,
        space = RakuraiTipAccount::SIZE,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_6: Account<'info, RakuraiTipAccount>,
    #[account(
        init,
        seeds = [RAKURAI_TIP_ACCOUNT_7_SEED],
        bump,
        payer = payer,
        space = RakuraiTipAccount::SIZE,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_7: Account<'info, RakuraiTipAccount>,

    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseRakuraiTipManager<'info> {
    /// singleton account
    #[account(
        mut,
        close = signer,
        seeds = [TIP_MANAGER_CONFIG_ACCOUNT_SEED],
        bump,
        rent_exempt = enforce
    )]
    pub tip_manager_config: Account<'info, TipManagerConfigAccount>,
    #[account(
        mut,
        close = signer,
        seeds = [RAKURAI_TIP_ACCOUNT_0_SEED],
        bump,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_0: Account<'info, RakuraiTipAccount>,
    #[account(
        mut,
        close = signer,
        seeds = [RAKURAI_TIP_ACCOUNT_1_SEED],
        bump,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_1: Account<'info, RakuraiTipAccount>,
    #[account(
        mut,
        close = signer,
        seeds = [RAKURAI_TIP_ACCOUNT_2_SEED],
        bump,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_2: Account<'info, RakuraiTipAccount>,
    #[account(
        mut,
        close = signer,
        seeds = [RAKURAI_TIP_ACCOUNT_3_SEED],
        bump,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_3: Account<'info, RakuraiTipAccount>,
    #[account(
        mut,
        close = signer,
        seeds = [RAKURAI_TIP_ACCOUNT_4_SEED],
        bump,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_4: Account<'info, RakuraiTipAccount>,
    #[account(
        mut,
        close = signer,
        seeds = [RAKURAI_TIP_ACCOUNT_5_SEED],
        bump,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_5: Account<'info, RakuraiTipAccount>,
    #[account(
        mut,
        close = signer,
        seeds = [RAKURAI_TIP_ACCOUNT_6_SEED],
        bump,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_6: Account<'info, RakuraiTipAccount>,
    #[account(
        mut,
        close = signer,
        seeds = [RAKURAI_TIP_ACCOUNT_7_SEED],
        bump,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_7: Account<'info, RakuraiTipAccount>,

    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl CloseRakuraiTipManager<'_> {
    fn auth(ctx: &Context<CloseRakuraiTipManager>) -> Result<()> {
        if ctx.accounts.tip_manager_config.authority != ctx.accounts.signer.key() {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct ChangeTipReceiver<'info> {
    #[account(
        mut,
        seeds = [TIP_MANAGER_CONFIG_ACCOUNT_SEED],
        bump = tip_manager_config.bumps.tip_manager_config,
        rent_exempt = enforce
    )]
    pub tip_manager_config: Account<'info, TipManagerConfigAccount>,

    /// CHECK: old_tip_receiver receives the funds in the RakuraiTipAccount accounts
    #[account(mut, constraint = old_tip_receiver.key() == tip_manager_config.validator_tip_receiver_account)]
    pub old_tip_receiver: AccountInfo<'info>,

    /// CHECK: any new, writable account is allowed as a tip receiver.
    #[account(mut)]
    pub new_tip_receiver: AccountInfo<'info>,

    /// CHECK: old_client receives a % of funds in the RakuraiTipAccount accounts
    #[account(mut, constraint = client_commission_account.key() == tip_manager_config.client_commission_account)]
    pub client_commission_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_0_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_0,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_0: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_1_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_1,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_1: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_2_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_2,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_2: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_3_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_3,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_3: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_4_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_4,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_4: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_5_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_5,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_5: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_6_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_6,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_6: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_7_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_7,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_7: Account<'info, RakuraiTipAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl<'info> ChangeTipReceiver<'info> {
    fn get_tip_accounts(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.rakurai_tip_account_0.to_account_info(),
            self.rakurai_tip_account_1.to_account_info(),
            self.rakurai_tip_account_2.to_account_info(),
            self.rakurai_tip_account_3.to_account_info(),
            self.rakurai_tip_account_4.to_account_info(),
            self.rakurai_tip_account_5.to_account_info(),
            self.rakurai_tip_account_6.to_account_info(),
            self.rakurai_tip_account_7.to_account_info(),
        ]
    }
}

#[derive(Accounts)]
pub struct ChangeTipReceiverV1<'info> {
    #[account(
        mut,
        seeds = [TIP_MANAGER_CONFIG_ACCOUNT_SEED],
        bump = tip_manager_config.bumps.tip_manager_config,
        rent_exempt = enforce
    )]
    pub tip_manager_config: Account<'info, TipManagerConfigAccount>,

    /// CHECK: old_tip_receiver receives the funds in the RakuraiTipAccount accounts
    #[account(mut, constraint = old_tip_receiver.key() == tip_manager_config.validator_tip_receiver_account)]
    pub old_tip_receiver: AccountInfo<'info>,

    /// Rakurai tip revenue share PDA (`TipsCollectionAccount` / TCA) for this validator vote.
    #[account(mut, owner = reward_distribution::ID)]
    pub new_tip_receiver: Account<'info, TipsCollectionAccount>,

    /// CHECK: receives commission; must match tip-manager global `client_commission_account`.
    #[account(mut, constraint = client_commission_account.key() == tip_manager_config.client_commission_account)]
    pub client_commission_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_0_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_0,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_0: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_1_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_1,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_1: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_2_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_2,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_2: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_3_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_3,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_3: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_4_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_4,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_4: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_5_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_5,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_5: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_6_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_6,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_6: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_7_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_7,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_7: Account<'info, RakuraiTipAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: PDA that signs the `reward_distribution::record_revenue` CPI. Must be set as the
    /// receiving TCA's `record_authority`; the seeds constraint guarantees only this program can sign.
    #[account(seeds = [RECORD_AUTHORITY_SEED], bump)]
    pub record_authority: UncheckedAccount<'info>,
}

impl ChangeTipReceiverV1<'_> {
    /// Remaining accounts: `[0]` enabled RAA PDA; `[1]` reward distribution program id;
    fn auth(ctx: &Context<ChangeTipReceiverV1>) -> Result<()> {
        use anchor_lang::AccountDeserialize;
        let (expected, _) = crate::sdk::derive_rakurai_tip_collection_address(
            &reward_distribution::ID,
            &ctx.accounts.new_tip_receiver.validator_vote,
        );
        if ctx.accounts.new_tip_receiver.key() != expected {
            return Err(Unauthorized.into());
        }

        require_gte!(
            ctx.remaining_accounts.len(),
            2,
            RakuraiTipManagerError::Unauthorized
        );

        let raa_info = &ctx.remaining_accounts[0];
        let (expected_raa, expected_bump) = Pubkey::find_program_address(
            &[
                RakuraiActivationAccount::SEED,
                ctx.accounts.signer.key.as_ref(),
            ],
            &rakurai_activation::ID,
        );
        require!(
            raa_info.key() == expected_raa,
            RakuraiTipManagerError::Unauthorized
        );

        let raa = RakuraiActivationAccount::try_deserialize(&mut &raa_info.data.borrow()[..])
            .map_err(|_| RakuraiTipManagerError::Unauthorized)?;
        require!(
            raa.bump == expected_bump,
            RakuraiTipManagerError::Unauthorized
        );
        require!(
            raa.validator_authority == ctx.accounts.signer.key(),
            RakuraiTipManagerError::Unauthorized
        );
        require!(
            raa.is_enabled,
            RakuraiTipManagerError::RakuraiSchedulerNotEnabled
        );

        let reward_distribution_program = &ctx.remaining_accounts[1];
        require!(
            reward_distribution_program.key() == reward_distribution::ID,
            RakuraiTipManagerError::Unauthorized
        );

        // Note: TCA `record_authority` is validated at execution time to gate the
        // `record_revenue` CPI. A mismatch skips only recording, not the payout.

        Ok(())
    }
}

impl<'info> ChangeTipReceiverV1<'info> {
    fn get_tip_accounts(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.rakurai_tip_account_0.to_account_info(),
            self.rakurai_tip_account_1.to_account_info(),
            self.rakurai_tip_account_2.to_account_info(),
            self.rakurai_tip_account_3.to_account_info(),
            self.rakurai_tip_account_4.to_account_info(),
            self.rakurai_tip_account_5.to_account_info(),
            self.rakurai_tip_account_6.to_account_info(),
            self.rakurai_tip_account_7.to_account_info(),
        ]
    }
}

/// Mirror of [`ChangeTipReceiverV1`] for TCAV1 (`REVENUE_SHARE_V1`).
/// Commission is taken from the **new** TCAV1 and synced onto tip-manager config.
#[derive(Accounts)]
pub struct ChangeTipReceiverV2<'info> {
    #[account(
        mut,
        seeds = [TIP_MANAGER_CONFIG_ACCOUNT_SEED],
        bump = tip_manager_config.bumps.tip_manager_config,
        rent_exempt = enforce
    )]
    pub tip_manager_config: Account<'info, TipManagerConfigAccount>,

    /// CHECK: old_tip_receiver receives the funds in the RakuraiTipAccount accounts
    #[account(mut, constraint = old_tip_receiver.key() == tip_manager_config.validator_tip_receiver_account)]
    pub old_tip_receiver: AccountInfo<'info>,

    /// Rakurai tip TCAV1 PDA for this validator vote.
    #[account(mut, owner = reward_distribution::ID)]
    pub new_tip_receiver: Account<'info, TipsCollectionAccountV1>,

    /// CHECK: receives commission; must match tip-manager global `client_commission_account`.
    #[account(mut, constraint = client_commission_account.key() == tip_manager_config.client_commission_account)]
    pub client_commission_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_0_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_0,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_0: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_1_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_1,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_1: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_2_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_2,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_2: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_3_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_3,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_3: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_4_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_4,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_4: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_5_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_5,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_5: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_6_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_6,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_6: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_7_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_7,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_7: Account<'info, RakuraiTipAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: PDA that signs the `reward_distribution::record_revenue_v1` CPI.
    #[account(seeds = [RECORD_AUTHORITY_SEED], bump)]
    pub record_authority: UncheckedAccount<'info>,
}

impl ChangeTipReceiverV2<'_> {
    /// Remaining accounts: `[0]` enabled RAA PDA; `[1]` reward distribution program id.
    fn auth(ctx: &Context<ChangeTipReceiverV2>) -> Result<()> {
        use anchor_lang::AccountDeserialize;
        let (expected, _) = crate::sdk::derive_rakurai_tip_collection_v1_address(
            &reward_distribution::ID,
            &ctx.accounts.new_tip_receiver.validator_vote,
        );
        if ctx.accounts.new_tip_receiver.key() != expected {
            return Err(Unauthorized.into());
        }

        require_gte!(
            ctx.remaining_accounts.len(),
            2,
            RakuraiTipManagerError::Unauthorized
        );

        let raa_info = &ctx.remaining_accounts[0];
        let (expected_raa, expected_bump) = Pubkey::find_program_address(
            &[
                RakuraiActivationAccount::SEED,
                ctx.accounts.signer.key.as_ref(),
            ],
            &rakurai_activation::ID,
        );
        require!(
            raa_info.key() == expected_raa,
            RakuraiTipManagerError::Unauthorized
        );

        let raa = RakuraiActivationAccount::try_deserialize(&mut &raa_info.data.borrow()[..])
            .map_err(|_| RakuraiTipManagerError::Unauthorized)?;
        require!(
            raa.bump == expected_bump,
            RakuraiTipManagerError::Unauthorized
        );
        require!(
            raa.validator_authority == ctx.accounts.signer.key(),
            RakuraiTipManagerError::Unauthorized
        );
        require!(
            raa.is_enabled,
            RakuraiTipManagerError::RakuraiSchedulerNotEnabled
        );

        let reward_distribution_program = &ctx.remaining_accounts[1];
        require!(
            reward_distribution_program.key() == reward_distribution::ID,
            RakuraiTipManagerError::Unauthorized
        );

        Ok(())
    }
}

impl<'info> ChangeTipReceiverV2<'info> {
    fn get_tip_accounts(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.rakurai_tip_account_0.to_account_info(),
            self.rakurai_tip_account_1.to_account_info(),
            self.rakurai_tip_account_2.to_account_info(),
            self.rakurai_tip_account_3.to_account_info(),
            self.rakurai_tip_account_4.to_account_info(),
            self.rakurai_tip_account_5.to_account_info(),
            self.rakurai_tip_account_6.to_account_info(),
            self.rakurai_tip_account_7.to_account_info(),
        ]
    }
}

#[derive(Accounts)]
pub struct ChangeClient<'info> {
    #[account(
        mut,
        seeds = [TIP_MANAGER_CONFIG_ACCOUNT_SEED],
        bump = tip_manager_config.bumps.tip_manager_config,
        rent_exempt = enforce
    )]
    pub tip_manager_config: Account<'info, TipManagerConfigAccount>,

    /// CHECK: old_tip_receiver receives the funds in the RakuraiTipAccount accounts
    #[account(mut, constraint = validator_tip_receiver_account.key() == tip_manager_config.validator_tip_receiver_account)]
    pub validator_tip_receiver_account: AccountInfo<'info>,

    /// CHECK: old_client receives a % of funds in the RakuraiTipAccount accounts
    #[account(mut, constraint = old_client.key() == tip_manager_config.client_commission_account)]
    pub old_client: AccountInfo<'info>,

    /// CHECK: any new, writable account is allowed as client
    #[account(mut)]
    pub new_client: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_0_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_0,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_0: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_1_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_1,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_1: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_2_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_2,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_2: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_3_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_3,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_3: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_4_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_4,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_4: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_5_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_5,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_5: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_6_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_6,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_6: Account<'info, RakuraiTipAccount>,

    #[account(
        mut,
        seeds = [RAKURAI_TIP_ACCOUNT_7_SEED],
        bump = tip_manager_config.bumps.rakurai_tip_account_7,
        rent_exempt = enforce
    )]
    pub rakurai_tip_account_7: Account<'info, RakuraiTipAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl ChangeClient<'_> {
    fn auth(ctx: &Context<ChangeClient>) -> Result<()> {
        if ctx.accounts.tip_manager_config.authority != ctx.accounts.signer.key() {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}
impl<'info> ChangeClient<'info> {
    fn get_tip_accounts(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.rakurai_tip_account_0.to_account_info(),
            self.rakurai_tip_account_1.to_account_info(),
            self.rakurai_tip_account_2.to_account_info(),
            self.rakurai_tip_account_3.to_account_info(),
            self.rakurai_tip_account_4.to_account_info(),
            self.rakurai_tip_account_5.to_account_info(),
            self.rakurai_tip_account_6.to_account_info(),
            self.rakurai_tip_account_7.to_account_info(),
        ]
    }
}

/// State Accounts

/// Singleton configuration account for the Rakurai Tip Manager.
#[account]
#[derive(Default)]
pub struct TipManagerConfigAccount {
    /// Authorized updater of the config.
    pub authority: Pubkey,

    /// Account receiving validator tips
    pub validator_tip_receiver_account: Pubkey,

    /// Client commission account
    pub client_commission_account: Pubkey,

    /// Commission in basis points
    pub client_commission_bps: u64,

    /// PDA bump seeds
    pub bumps: RakuraiTipManagerBumps,
}

impl TipManagerConfigAccount {
    pub const SIZE: usize = 8 + 32 + 32 + 32 + 8 + RakuraiTipManagerBumps::SIZE;
}

/// Account that temporarily holds tips.
/// Eight accounts are maintained to reduce account write-lock contention.
#[account]
#[derive(Default)]
pub struct RakuraiTipAccount {}

impl RakuraiTipAccount {
    pub const SIZE: usize = 8;

    /// Collects per-account drainable tips without modifying lamports.
    /// Returns `(total, per_account_tips)` so the caller can drain later
    /// via `drain_collected` without recomputing rent.
    fn collect_tips(rent: &Rent, accounts: &[AccountInfo]) -> Result<(u64, Vec<u64>)> {
        let mut total = 0u64;
        let mut per_account = Vec::with_capacity(accounts.len());
        for account in accounts {
            let tips = account
                .lamports()
                .checked_sub(rent.minimum_balance(account.data_len()))
                .ok_or(ArithmeticError)?;
            per_account.push(tips);
            total = total.checked_add(tips).ok_or(ArithmeticError)?;
        }
        Ok((total, per_account))
    }

    /// Drains precomputed tip amounts from each account.
    fn drain_collected(accounts: &[AccountInfo], per_account: &[u64]) -> Result<()> {
        for (account, &tips) in accounts.iter().zip(per_account) {
            if tips > 0 {
                **account.try_borrow_mut_lamports()? -= tips;
            }
        }
        Ok(())
    }

    /// Single-pass collect + drain. Returns total drained.
    fn drain_accounts(rent: &Rent, accounts: &[AccountInfo]) -> Result<u64> {
        let (total, per_account) = Self::collect_tips(rent, accounts)?;
        Self::drain_collected(accounts, &per_account)?;
        Ok(total)
    }

    fn close_account(authority: &Signer, account: &AccountInfo) -> Result<u64> {
        // Transfer all lamports from config to authority
        let lamports_to_reclaim = account.to_account_info().lamports();
        **account.to_account_info().try_borrow_mut_lamports()? = 0;
        **authority.try_borrow_mut_lamports()? = authority
            .lamports()
            .checked_add(lamports_to_reclaim)
            .ok_or(ArithmeticError)?;
        Ok(lamports_to_reclaim)
    }
}

/// Events
#[event]
pub struct TipsClaimedEvent {
    pub validator_tip_receiver_account: Pubkey,
    pub tip_receiver_amount: u64,
    pub client_commission_account: Pubkey,
    pub client_amount: u64,
}

#[event]
pub struct TipsManagerCloseEvent {
    pub close_authority: Pubkey,
    pub lamports_reclaimed: u64,
}
