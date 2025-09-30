use crate::constants::*;
use crate::errors::StarInvestorFeesError;
use crate::state::{DistributionProgress, PolicyConfig};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

#[derive(Accounts)]
#[instruction(vault_id: [u8; 32])]
pub struct FinalizeDayDistribution<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,

    /// Policy configuration
    #[account(
        seeds = [POLICY_SEED, vault_id.as_ref()],
        bump = policy.bump
    )]
    pub policy: Account<'info, PolicyConfig>,

    /// Distribution progress tracking
    #[account(
        mut,
        seeds = [PROGRESS_SEED, vault_id.as_ref()],
        bump = progress.bump
    )]
    pub progress: Account<'info, DistributionProgress>,

    /// Position owner PDA
    /// CHECK: Seeds validated
    #[account(
        seeds = [VAULT_SEED, vault_id.as_ref(), POSITION_OWNER_SEED],
        bump
    )]
    pub position_owner_pda: UncheckedAccount<'info>,

    /// Program treasury ATA
    #[account(
        mut,
        seeds = [TREASURY_SEED, vault_id.as_ref()],
        bump,
        token::mint = policy.quote_mint,
        token::authority = position_owner_pda
    )]
    pub treasury_ata: Account<'info, TokenAccount>,

    /// Creator's quote ATA (receives remainder)
    #[account(
        mut,
        constraint = creator_ata.mint == policy.quote_mint,
        constraint = creator_ata.owner == policy.creator
    )]
    pub creator_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<FinalizeDayDistribution>, vault_id: [u8; 32]) -> Result<()> {
    let progress = &mut ctx.accounts.progress;
    let current_ts = Clock::get()?.unix_timestamp;

    // Ensure day is not already finalized
    require!(
        !progress.day_finalized,
        StarInvestorFeesError::DayAlreadyFinalized
    );

    // Calculate remainder to send to creator
    let total_claimed = progress.daily_claimed_amount + progress.carry_over_dust;
    let distributed_to_investors = progress.daily_distributed_to_investors;

    let remainder = total_claimed
        .checked_sub(distributed_to_investors)
        .ok_or(StarInvestorFeesError::ArithmeticUnderflow)?;

    // Transfer remainder to creator if > 0
    if remainder > 0 {
        let position_owner_bump = ctx.bumps.position_owner_pda;
        let seeds = &[
            VAULT_SEED,
            vault_id.as_ref(),
            POSITION_OWNER_SEED,
            &[position_owner_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.treasury_ata.to_account_info(),
                to: ctx.accounts.creator_ata.to_account_info(),
                authority: ctx.accounts.position_owner_pda.to_account_info(),
            },
            signer_seeds,
        );

        token::transfer(transfer_ctx, remainder)?;

        progress.daily_distributed_to_creator = remainder;
    }

    // Mark day as finalized
    progress.day_finalized = true;

    // Emit event
    emit!(CreatorPayoutDayClosed {
        vault_id,
        creator: ctx.accounts.policy.creator,
        amount_paid: remainder,
        day_start: progress.current_day_start,
        day_end: current_ts,
        timestamp: current_ts,
    });

    msg!("Day finalized. Remainder {} sent to creator", remainder);

    Ok(())
}
