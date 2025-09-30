use crate::constants::*;
use crate::errors::StarInvestorFeesError;
use crate::state::{DistributionProgress, PolicyConfig};
use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::Mint;

#[derive(Accounts)]
#[instruction(vault_id: [u8; 32])]
pub struct InitializePolicy<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Creator wallet (will receive remainder fees)
    /// CHECK: Address stored in policy
    pub creator: UncheckedAccount<'info>,

    /// Policy configuration account
    #[account(
        init,
        payer = authority,
        space = PolicyConfig::LEN,
        seeds = [POLICY_SEED, vault_id.as_ref()],
        bump
    )]
    pub policy: Account<'info, PolicyConfig>,

    /// Distribution progress tracking
    #[account(
        init,
        payer = authority,
        space = DistributionProgress::LEN,
        seeds = [PROGRESS_SEED, vault_id.as_ref()],
        bump
    )]
    pub progress: Account<'info, DistributionProgress>,

    /// Quote mint
    pub quote_mint: InterfaceAccount<'info, Mint>,

    /// Pool address
    /// CHECK: Stored in policy for validation
    pub pool: UncheckedAccount<'info>,

    /// Honorary position
    /// CHECK: Stored in policy for validation
    pub position: UncheckedAccount<'info>,

    /// Token program (Token2022)
    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitializePolicy>,
    vault_id: [u8; 32],
    investor_fee_share_bps: u16,
    daily_cap_lamports: Option<u64>,
    min_payout_lamports: u64,
    y0_total_allocation: u64,
) -> Result<()> {
    // Validate fee share
    require!(
        investor_fee_share_bps <= MAX_INVESTOR_FEE_SHARE_BPS,
        StarInvestorFeesError::InvalidFeeShareBps
    );

    // Initialize policy
    let policy = &mut ctx.accounts.policy;
    policy.vault_id = vault_id;
    policy.authority = ctx.accounts.authority.key();
    policy.creator = ctx.accounts.creator.key();
    policy.investor_fee_share_bps = investor_fee_share_bps;
    policy.daily_cap_lamports = daily_cap_lamports;
    policy.min_payout_lamports = min_payout_lamports;
    policy.y0_total_allocation = y0_total_allocation;
    policy.quote_mint = ctx.accounts.quote_mint.key();
    policy.pool = ctx.accounts.pool.key();
    policy.position = ctx.accounts.position.key();
    policy.paused = false;
    policy.bump = ctx.bumps.policy;

    // Initialize progress
    let progress = &mut ctx.accounts.progress;
    progress.vault_id = vault_id;
    progress.last_distribution_ts = 0;
    progress.current_day_start = 0;
    progress.daily_claimed_amount = 0;
    progress.daily_distributed_to_investors = 0;
    progress.daily_distributed_to_creator = 0;
    progress.current_page = 0;
    progress.total_pages = 0;
    progress.carry_over_dust = 0;
    progress.day_finalized = false;
    progress.bump = ctx.bumps.progress;

    msg!("Policy initialized for vault: {:?}", vault_id);
    msg!("Investor fee share: {} bps", investor_fee_share_bps);
    msg!("Y0 allocation: {}", y0_total_allocation);

    emit!(PolicyUpdated {
        vault_id,
        investor_fee_share_bps,
        daily_cap_lamports,
        min_payout_lamports,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
