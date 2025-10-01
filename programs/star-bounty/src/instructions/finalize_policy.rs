use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::StarInvestorFeesError;
use crate::state::PolicyConfig;

#[derive(Accounts)]
#[instruction(vault_id: [u8; 32])]
pub struct UpdatePolicy<'info> {
    #[account(
        constraint = authority.key() == policy.authority @ StarInvestorFeesError::InvalidAuthority
    )]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [POLICY_SEED, vault_id.as_ref()],
        bump = policy.bump
    )]
    pub policy: Account<'info, PolicyConfig>,
}

pub fn handler(
    ctx: Context<UpdatePolicy>,
    _vault_id: [u8; 32],
    investor_fee_share_bps: Option<u16>,
    daily_cap_lamports: Option<Option<u64>>,
    min_payout_lamports: Option<u64>,
) -> Result<()> {
    let policy = &mut ctx.accounts.policy;

    // Update investor fee share if provided
    if let Some(bps) = investor_fee_share_bps {
        require!(
            bps <= MAX_INVESTOR_FEE_SHARE_BPS,
            StarInvestorFeesError::InvalidFeeShareBps
        );
        policy.investor_fee_share_bps = bps;
    }

    // Update daily cap if provided
    if let Some(cap) = daily_cap_lamports {
        policy.daily_cap_lamports = cap;
    }

    // Update minimum payout if provided
    if let Some(min) = min_payout_lamports {
        policy.min_payout_lamports = min;
    }

    emit!(PolicyUpdated {
        vault_id: policy.vault_id,
        investor_fee_share_bps: policy.investor_fee_share_bps,
        daily_cap_lamports: policy.daily_cap_lamports,
        min_payout_lamports: policy.min_payout_lamports,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Policy updated successfully");

    Ok(())
}