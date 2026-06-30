#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use rakurai_activation::state::RakuraiActivationAccount;
use rakurai_vote_state::VoteState;

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::RakuraiTipManagerError::{ArithmeticError, RakuraiSchedulerNotEnabled, Unauthorized};

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Rakurai Tip Manager Program",
    project_url: "https://github.com/rakurai-io/rakurai-validator",
    contacts: "https://rakurai.io/company",
    policy: "https://rakurai.io/faqs"
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

/// Account discriminator size
pub const HEADER: usize = 8;
const MAX_COMMISSION_BPS: u64 = 10_000;

/// Partner label for the Rakurai tip-share vault (`name` field in PDA seeds).
pub const RAKURAI_PARTNER_TIP_SHARE_NAME: [u8; 32] = {
    let mut name = [0u8; 32];
    name[0] = b'R';
    name[1] = b'a';
    name[2] = b'k';
    name[3] = b'u';
    name[4] = b'r';
    name[5] = b'a';
    name[6] = b'i';
    name
};

/// Rakurai Tip Manager Program: users send tips to one of eight tip accounts, validators periodically drain them
/// and tips are split between the configured tip receiver and an block builder commission account.
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
        cfg.block_builder_commission_account = ctx.accounts.payer.key();
        cfg.block_builder_commission_bps = MAX_COMMISSION_BPS;
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

    /// Changes the active tip receiver by first draining all pending tips (giving the old tip receiver and block builder
    /// their respective shares) and then setting the new tip receiver for future tips.
    /// Only a Rakurai-enabled validator (identity signer + enabled RAA) may invoke this.
    pub fn change_tip_receiver(ctx: Context<ChangeTipReceiver>) -> Result<()> {
        ChangeTipReceiver::auth(&ctx)?;

        let total_tips = RakuraiTipAccount::drain_accounts(ctx.accounts.get_tip_accounts())?;

        let block_builder_fee = total_tips
            .checked_mul(ctx.accounts.tip_manager_config.block_builder_commission_bps)
            .ok_or(ArithmeticError)?
            .checked_div(MAX_COMMISSION_BPS)
            .ok_or(ArithmeticError)?;

        let validator_fee = total_tips
            .checked_sub(block_builder_fee)
            .ok_or(ArithmeticError)?;

        if validator_fee > 0 {
            **ctx.accounts.old_tip_receiver.try_borrow_mut_lamports()? += validator_fee;
        }

        if block_builder_fee > 0 {
            **ctx
                .accounts
                .block_builder_commission_account
                .try_borrow_mut_lamports()? += block_builder_fee;
        }

        if block_builder_fee > 0 || validator_fee > 0 {
            emit!(TipsClaimedEvent {
                validator_tip_receiver_account: ctx.accounts.old_tip_receiver.key(),
                tip_receiver_amount: validator_fee,
                block_builder_commission_account: ctx
                    .accounts
                    .block_builder_commission_account
                    .key(),
                block_builder_amount: block_builder_fee,
            });
        }

        ctx.accounts
            .tip_manager_config
            .validator_tip_receiver_account = ctx.accounts.new_tip_receiver.key();

        Ok(())
    }

    /// Changes the block builder and its commission by first draining all pending tips (distributing shares to the tip receiver
    /// and old block builder) and then setting the new block builder and its commission.
    pub fn change_block_builder(
        ctx: Context<ChangeBlockBuilder>,
        block_builder_commission_bps: u64,
    ) -> Result<()> {
        ChangeBlockBuilder::auth(&ctx)?;
        require_gte!(
            MAX_COMMISSION_BPS,
            block_builder_commission_bps,
            RakuraiTipManagerError::MaxCommissionBpsExceeded
        );
        let total_tips = RakuraiTipAccount::drain_accounts(ctx.accounts.get_tip_accounts())?;

        let block_builder_fee = total_tips
            .checked_mul(ctx.accounts.tip_manager_config.block_builder_commission_bps)
            .ok_or(ArithmeticError)?
            .checked_div(MAX_COMMISSION_BPS)
            .ok_or(ArithmeticError)?;

        let validator_fee = total_tips
            .checked_sub(block_builder_fee)
            .ok_or(ArithmeticError)?;

        if validator_fee > 0 {
            **ctx
                .accounts
                .validator_tip_receiver_account
                .try_borrow_mut_lamports()? += validator_fee;
        }

        if block_builder_fee > 0 {
            **ctx.accounts.old_block_builder.try_borrow_mut_lamports()? += block_builder_fee;
        }

        if block_builder_fee > 0 || validator_fee > 0 {
            emit!(TipsClaimedEvent {
                validator_tip_receiver_account: ctx.accounts.validator_tip_receiver_account.key(),
                tip_receiver_amount: validator_fee,
                block_builder_commission_account: ctx.accounts.old_block_builder.key(),
                block_builder_amount: block_builder_fee,
            });
        }

        ctx.accounts
            .tip_manager_config
            .block_builder_commission_account = ctx.accounts.new_block_builder.key();
        ctx.accounts.tip_manager_config.block_builder_commission_bps = block_builder_commission_bps;

        Ok(())
    }
}

/// Errors
#[error_code]
pub enum RakuraiTipManagerError {
    #[msg("Encountered an arithmetic under/overflow error.")]
    ArithmeticError,

    #[msg("Block Builder commission basis points must be less than or equal to 10_000")]
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

    #[account(
        seeds = [RakuraiActivationAccount::SEED, signer.key().as_ref()],
        bump = rakurai_activation_account.bump,
        seeds::program = rakurai_activation::ID,
        constraint = rakurai_activation_account.validator_authority == signer.key(),
        constraint = rakurai_activation_account.is_enabled @ RakuraiSchedulerNotEnabled,
    )]
    pub rakurai_activation_account: Account<'info, RakuraiActivationAccount>,

    /// CHECK: validator vote account; node pubkey must match signer.
    pub validator_vote_account: AccountInfo<'info>,

    /// CHECK: old_tip_receiver receives the funds in the RakuraiTipAccount accounts
    #[account(mut, constraint = old_tip_receiver.key() == tip_manager_config.validator_tip_receiver_account)]
    pub old_tip_receiver: AccountInfo<'info>,

    /// CHECK: reward_distribution program id for partner tip-share PDA derivation.
    pub reward_distribution_program: AccountInfo<'info>,

    /// Rakurai partner tip-share PDA for this validator vote.
    #[account(
        mut,
        constraint = new_tip_receiver.owner == reward_distribution_program.key @ Unauthorized,
        constraint = {
            let (expected, _) = crate::sdk::derive_rakurai_partner_tip_share_address(
                &reward_distribution_program.key(),
                &validator_vote_account.key(),
            );
            new_tip_receiver.key() == expected
        } @ Unauthorized,
    )]
    pub new_tip_receiver: AccountInfo<'info>,

    /// CHECK: old_block_builder receives a % of funds in the RakuraiTipAccount accounts
    #[account(mut, constraint = block_builder_commission_account.key() == tip_manager_config.block_builder_commission_account)]
    pub block_builder_commission_account: AccountInfo<'info>,

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

impl ChangeTipReceiver<'_> {
    fn auth(ctx: &Context<ChangeTipReceiver>) -> Result<()> {
        if ctx.accounts.validator_vote_account.owner != &solana_program::vote::program::id() {
            return Err(Unauthorized.into());
        }

        let node_pubkey = VoteState::deserialize_node_pubkey(&ctx.accounts.validator_vote_account)
            .map_err(|_| Unauthorized)?;

        if node_pubkey != *ctx.accounts.signer.key {
            return Err(Unauthorized.into());
        }

        Ok(())
    }
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
pub struct ChangeBlockBuilder<'info> {
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

    /// CHECK: old_block_builder receives a % of funds in the RakuraiTipAccount accounts
    #[account(mut, constraint = old_block_builder.key() == tip_manager_config.block_builder_commission_account)]
    pub old_block_builder: AccountInfo<'info>,

    /// CHECK: any new, writable account is allowed as block builder
    #[account(mut)]
    pub new_block_builder: AccountInfo<'info>,

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

impl ChangeBlockBuilder<'_> {
    fn auth(ctx: &Context<ChangeBlockBuilder>) -> Result<()> {
        if ctx.accounts.tip_manager_config.authority != ctx.accounts.signer.key() {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}
impl<'info> ChangeBlockBuilder<'info> {
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

    /// Block builder commission account
    pub block_builder_commission_account: Pubkey,

    /// Commission in basis points
    pub block_builder_commission_bps: u64,

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

    /// Drains all provided tip accounts while preserving rent exemption.
    fn drain_accounts(accounts: Vec<AccountInfo>) -> Result<u64> {
        let mut total = 0u64;
        for account in accounts {
            total = total
                .checked_add(Self::drain_account(&account)?)
                .ok_or(ArithmeticError)?;
        }
        Ok(total)
    }

    fn drain_account(account: &AccountInfo) -> Result<u64> {
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(account.data_len());

        let tips = account
            .lamports()
            .checked_sub(min_rent)
            .ok_or(ArithmeticError)?;

        **account.try_borrow_mut_lamports()? -= tips;
        Ok(tips)
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
    pub block_builder_commission_account: Pubkey,
    pub block_builder_amount: u64,
}

#[event]
pub struct TipsManagerCloseEvent {
    pub close_authority: Pubkey,
    pub lamports_reclaimed: u64,
}
