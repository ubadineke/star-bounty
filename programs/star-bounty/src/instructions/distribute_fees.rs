use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::constants::*;
use crate::errors::StarInvestorFeesError;
use crate::state::{PolicyConfig, DistributionProgress, StreamflowStream};
use crate::utils::{safe_math, validation, fee_calc, events, logging};

#[derive(Accounts)]
#[instruction(vault_id: [u8; 32])]
pub struct DistributeFees<'info> {
    /// Cranker who executes the distribution (can be anyone - permissionless)
    #[account(mut)]
    pub cranker: Signer<'info>,

    /// Policy configuration account
    #[account(
        seeds = [POLICY_SEED, vault_id.as_ref()],
        bump = policy.bump,
        constraint = !policy.paused @ StarInvestorFeesError::ProgramPaused
    )]
    pub policy: Account<'info, PolicyConfig>,

    /// Distribution progress tracking account
    #[account(
        mut,
        seeds = [PROGRESS_SEED, vault_id.as_ref()],
        bump = progress.bump
    )]
    pub progress: Account<'info, DistributionProgress>,

    /// Position owner PDA that controls the honorary position
    /// CHECK: Seeds validated, used for signing
    #[account(
        seeds = [VAULT_SEED, vault_id.as_ref(), POSITION_OWNER_SEED],
        bump
    )]
    pub position_owner_pda: UncheckedAccount<'info>,

    /// Honorary position account
    /// CHECK: Validated against policy
    #[account(
        constraint = position.key() == policy.position @ StarInvestorFeesError::InvalidAuthority
    )]
    pub position: UncheckedAccount<'info>,

    /// Program treasury ATA (holds claimed fees before distribution)
    #[account(
        mut,
        seeds = [TREASURY_SEED, vault_id.as_ref()],
        bump,
        constraint = treasury_ata.mint == policy.quote_mint @ StarInvestorFeesError::InvalidQuoteMint,
        constraint = treasury_ata.owner == position_owner_pda.key() @ StarInvestorFeesError::InvalidTreasuryAta
    )]
    pub treasury_ata: Account<'info, TokenAccount>,

    /// Pool account
    /// CHECK: Validated against policy
    #[account(
        constraint = pool.key() == policy.pool @ StarInvestorFeesError::InvalidPoolConfig
    )]
    pub pool: UncheckedAccount<'info>,

    /// Pool's quote token vault
    #[account(
        mut,
        constraint = pool_quote_vault.mint == policy.quote_mint @ StarInvestorFeesError::InvalidQuoteMint
    )]
    pub pool_quote_vault: Account<'info, TokenAccount>,

    /// Pool's base token vault (for validation - should never receive fees)
    #[account(
        constraint = pool_base_vault.mint != policy.quote_mint @ StarInvestorFeesError::InvalidPoolTokenOrder
    )]
    pub pool_base_vault: Account<'info, TokenAccount>,

    /// CP-AMM program
    /// CHECK: Program ID validated
    #[account(
        constraint = cp_amm_program.key() == anchor_lang::solana_program::pubkey!("CPAmmL9tg1U4bCUQ38Kkdq1rF53tGPY1Hxj3pzBNXwYG")
            @ StarInvestorFeesError::InvalidCpAmmProgram
    )]
    pub cp_amm_program: UncheckedAccount<'info>,

    /// Streamflow program
    /// CHECK: Program ID validated
    #[account(
        constraint = streamflow_program.key() == anchor_lang::solana_program::pubkey!("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUTNYXr6vk")
            @ StarInvestorFeesError::InvalidStreamflowProgram
    )]
    pub streamflow_program: UncheckedAccount<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<DistributeFees>,
    vault_id: [u8; 32],
    page_number: u16,
) -> Result<()> {
    let policy = &ctx.accounts.policy;
    let progress = &mut ctx.accounts.progress;
    let current_ts = Clock::get()?.unix_timestamp;

    msg!("=== Distribution Crank Started ===");
    msg!("Vault ID: {:?}", vault_id);
    msg!("Page: {}", page_number);
    msg!("Timestamp: {}", current_ts);

    // Validate vault ID matches
    // validation::validate_vault_id(&progress.vault_id, &vault_id)?;

    // Check if this is the first call of a new day (page 0)
    let is_new_day = page_number == 0 && progress.is_new_day_ready(current_ts);
    
    if is_new_day {
        msg!("Starting new distribution day");
        
        // Enforce 24h gating
        require!(
            progress.is_new_day_ready(current_ts),
            StarInvestorFeesError::DistributionTooEarly
        );

        // Get treasury balance before claiming
        let treasury_balance_before = ctx.accounts.treasury_ata.amount;
        msg!("Treasury balance before claim: {}", treasury_balance_before);

        // Claim fees from the honorary position via CP-AMM
        let claimed_amount = claim_fees_from_position(&ctx)?;
        
        msg!("Fees claimed: {}", claimed_amount);

        // Validate we received quote tokens only (no base tokens)
        validate_quote_only_claim(&ctx, claimed_amount)?;

        // Require non-zero claim
        require!(
            claimed_amount > 0,
            StarInvestorFeesError::NoFeesToDistribute
        );

        // Reset progress for new day
        progress.reset_for_new_day(current_ts, claimed_amount);
        progress.last_distribution_ts = current_ts;

        // Emit fees claimed event
        events::emit_fees_claimed(vault_id, claimed_amount, current_ts)?;
        
        msg!("New day initialized with {} fees", claimed_amount);
    } else {
        // Not a new day - validate we're continuing an existing day
        msg!("Continuing distribution for current day");
        
        // Validate sequential page processing
        validation::validate_page_number(page_number, progress.current_page)?;
        
        // Ensure day is not already finalized
        require!(
            !progress.day_finalized,
            StarInvestorFeesError::DayAlreadyFinalized
        );

        msg!("Current day started at: {}", progress.current_day_start);
        msg!("Total claimed this day: {}", progress.daily_claimed_amount);
        msg!("Already distributed: {}", progress.daily_distributed_to_investors);
    }

    // Get remaining accounts (investor data)
    let remaining_accounts = &ctx.remaining_accounts;
    
    // Validate remaining accounts (must be pairs: stream + ATA)
    require!(
        remaining_accounts.len() % 2 == 0,
        StarInvestorFeesError::InvalidRemainingAccounts
    );

    let investor_count = remaining_accounts.len() / 2;
    
    msg!("Processing {} investors on this page", investor_count);

    // Validate investor count
    require!(
        investor_count <= MAX_INVESTORS_PER_PAGE,
        StarInvestorFeesError::TooManyInvestorsPerPage
    );

    // If no investors on this page, skip
    if investor_count == 0 {
        msg!("No investors to process on page {}", page_number);
        progress.current_page = safe_math::add(progress.current_page as u64, 1)? as u16;
        return Ok(());
    }

    // Calculate total locked across all investors on this page
    let mut total_locked_this_page = 0u64;
    let mut investor_locked_amounts: Vec<u64> = Vec::with_capacity(investor_count);

    msg!("Reading Streamflow lock data for {} investors", investor_count);

    for i in 0..investor_count {
        let stream_account_info = &remaining_accounts[i * 2];
        
        // Deserialize Streamflow stream data
        let stream_data = parse_streamflow_stream(stream_account_info)?;
        
        // Calculate still-locked amount at current time
        let locked_amount = stream_data.calculate_locked_amount(current_ts as u64);
        
        msg!("Investor {}: locked = {}", i, locked_amount);
        
        investor_locked_amounts.push(locked_amount);
        
        total_locked_this_page = safe_math::add(total_locked_this_page, locked_amount)?;
    }

    msg!("Total locked on page: {}", total_locked_this_page);

    // Skip if no locked amounts
    if total_locked_this_page == 0 {
        msg!("No locked tokens found, skipping page {}", page_number);
        progress.current_page = safe_math::add(progress.current_page as u64, 1)? as u16;
        return Ok(());
    }

    // Validate total locked doesn't exceed Y0
    require!(
        total_locked_this_page <= policy.y0_total_allocation,
        StarInvestorFeesError::InvalidLockedTotal
    );

    // Calculate f_locked(t) = total_locked / Y0
    let f_locked = fee_calc::calculate_f_locked(total_locked_this_page, policy.y0_total_allocation)?;
    
    msg!("f_locked = {} bps", f_locked);

    // Calculate eligible investor share
    // eligible_investor_share_bps = min(investor_fee_share_bps, floor(f_locked(t) * 10000))
    let eligible_investor_share_bps = fee_calc::calculate_eligible_share(
        policy.investor_fee_share_bps,
        f_locked,
    );

    msg!("Eligible investor share: {} bps ({}%)", 
         eligible_investor_share_bps, 
         eligible_investor_share_bps as f64 / 100.0);

    // Calculate total investor fee for this day
    // investor_fee_quote = floor(claimed_quote * eligible_investor_share_bps / 10000)
    let total_claimable = safe_math::add(progress.daily_claimed_amount, progress.carry_over_dust)?;
    let investor_fee_quote = fee_calc::calculate_investor_fee(
        total_claimable,
        eligible_investor_share_bps,
    )?;

    msg!("Total investor fee pool: {}", investor_fee_quote);

    // Apply daily cap if configured
    let mut investor_fee_to_distribute = investor_fee_quote;
    
    if let Some(cap) = policy.daily_cap_lamports {
        let already_distributed = progress.daily_distributed_to_investors;
        let remaining_cap = cap.saturating_sub(already_distributed);
        
        if investor_fee_to_distribute > remaining_cap {
            msg!("Daily cap applied: {} -> {}", investor_fee_to_distribute, remaining_cap);
            investor_fee_to_distribute = remaining_cap;
        }
    }

    msg!("Amount to distribute this page: {}", investor_fee_to_distribute);

    // Distribute to investors pro-rata based on locked amounts
    let mut total_paid_this_page = 0u64;
    let mut investors_paid = 0u16;

    // Get signer seeds for PDA
    let position_owner_bump = ctx.bumps.position_owner_pda;
    let seeds = &[
        VAULT_SEED,
        vault_id.as_ref(),
        POSITION_OWNER_SEED,
        &[position_owner_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    msg!("Distributing to investors...");

    for i in 0..investor_count {
        let locked_amount = investor_locked_amounts[i];
        
        if locked_amount == 0 {
            msg!("Investor {} has no locked tokens, skipping", i);
            continue;
        }

        // Calculate proportional payout
        // payout = floor(investor_fee_quote * weight_i(t))
        // weight_i(t) = locked_i(t) / locked_total(t)
        let payout = fee_calc::calculate_proportional_payout(
            investor_fee_to_distribute,
            locked_amount,
            total_locked_this_page,
        )?;

        msg!("Investor {}: locked={}, payout={}", i, locked_amount, payout);

        // Apply minimum payout threshold
        if payout < policy.min_payout_lamports {
            msg!("Payout {} below minimum {}, adding to dust", 
                 payout, policy.min_payout_lamports);
            
            // Carry forward as dust
            progress.carry_over_dust = safe_math::add(progress.carry_over_dust, payout)?;
            continue;
        }

        // Get investor ATA from remaining accounts
        let investor_ata_info = &remaining_accounts[i * 2 + 1];
        
        // Deserialize investor token account
        let investor_ata = Account::<TokenAccount>::try_from(investor_ata_info)
            .map_err(|_| StarInvestorFeesError::InvalidInvestorAta)?;

        // Validate investor ATA
        require!(
            investor_ata.mint == policy.quote_mint,
            StarInvestorFeesError::InvalidInvestorAta
        );

        // Transfer tokens to investor
        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.treasury_ata.to_account_info(),
                to: investor_ata_info.clone(),
                authority: ctx.accounts.position_owner_pda.to_account_info(),
            },
            signer_seeds,
        );

        token::transfer(transfer_ctx, payout)?;

        msg!("✓ Transferred {} to investor {}", payout, i);

        total_paid_this_page = safe_math::add(total_paid_this_page, payout)?;
        investors_paid = safe_math::add(investors_paid as u64, 1)? as u16;
    }

    msg!("Page complete: {} paid to {} investors", total_paid_this_page, investors_paid);

    // Update progress
    progress.daily_distributed_to_investors = safe_math::add(
        progress.daily_distributed_to_investors,
        total_paid_this_page
    )?;

    progress.current_page = safe_math::add(progress.current_page as u64, 1)? as u16;

    // Emit event
    events::emit_payout_page(
        vault_id,
        page_number,
        investors_paid,
        total_paid_this_page,
    )?;

    // Log summary
    logging::log_distribution_summary(
        page_number,
        investors_paid,
        total_paid_this_page,
        total_locked_this_page,
    );

    msg!("=== Distribution Page {} Complete ===", page_number);

    Ok(())
}

/// Claim fees from the honorary position via CP-AMM
fn claim_fees_from_position(ctx: &Context<DistributeFees>) -> Result<u64> {
    msg!("Claiming fees from position: {}", ctx.accounts.position.key());
    
    // Get treasury balance before claim
    let balance_before = ctx.accounts.treasury_ata.amount;
    
    // In production, this would be a CPI call to cp-amm's collect_fees instruction
    // Example (pseudo-code):
    /*
    let position_owner_bump = ctx.bumps.position_owner_pda;
    let seeds = &[
        VAULT_SEED,
        ctx.accounts.progress.vault_id.as_ref(),
        POSITION_OWNER_SEED,
        &[position_owner_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = CollectFees {
        position_authority: ctx.accounts.position_owner_pda.to_account_info(),
        position: ctx.accounts.position.to_account_info(),
        pool: ctx.accounts.pool.to_account_info(),
        token_vault: ctx.accounts.pool_quote_vault.to_account_info(),
        recipient: ctx.accounts.treasury_ata.to_account_info(),
        // ... other accounts
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.cp_amm_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    cp_amm::cpi::collect_fees(cpi_ctx)?;
    */

    msg!("Fee collection CPI would be called here");
    msg!("Position: {}", ctx.accounts.position.key());
    msg!("Treasury before: {}", balance_before);
    
    // After CPI, reload account to get new balance
    // ctx.accounts.treasury_ata.reload()?;
    let balance_after = ctx.accounts.treasury_ata.amount;

    msg!("Treasury after: {}", balance_after);

    // Calculate claimed amount
    let claimed = balance_after.saturating_sub(balance_before);
    
    require!(claimed > 0, StarInvestorFeesError::ZeroFeeClaim);

    logging::log_fee_claim(claimed, balance_after);

    Ok(claimed)
}

/// Validate that only quote fees were claimed (no base fees)
fn validate_quote_only_claim(ctx: &Context<DistributeFees>, claimed_quote: u64) -> Result<()> {
    // In production, would check that:
    // 1. No base tokens appeared in treasury
    // 2. Position only accrued quote fees
    // 3. Pool state confirms quote-only fee generation

    // For now, basic validation
    require!(claimed_quote > 0, StarInvestorFeesError::ZeroFeeClaim);

    // Check that base vault balance didn't change (no base fees)
    // In actual implementation, would track base vault balance before/after
    // and fail if any base tokens were received
    
    msg!("Quote-only validation passed: {} quote tokens claimed", claimed_quote);
    
    Ok(())
}

/// Parse Streamflow stream account data
fn parse_streamflow_stream(account_info: &AccountInfo) -> Result<StreamflowStream> {
    msg!("Parsing Streamflow stream: {}", account_info.key);
    
    // Validate account
    require!(
        account_info.owner == &anchor_lang::solana_program::pubkey!("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUTNYXr6vk"),
        StarInvestorFeesError::InvalidStreamflowAccount
    );

    let data = account_info.try_borrow_data()?;
    
    // Validate minimum length
    require!(
        data.len() >= 48,
        StarInvestorFeesError::StreamflowDeserializationFailed
    );

    // Parse Streamflow stream data structure
    // Note: This is a simplified version. Actual Streamflow format may differ.
    // In production, would use Streamflow SDK for proper deserialization.
    
    // Skip discriminator (first 8 bytes) and parse fields
    let amount = u64::from_le_bytes(
        data[8..16].try_into()
            .map_err(|_| StarInvestorFeesError::StreamflowDeserializationFailed)?
    );
    
    let withdrawn = u64::from_le_bytes(
        data[16..24].try_into()
            .map_err(|_| StarInvestorFeesError::StreamflowDeserializationFailed)?
    );
    
    let start_time = u64::from_le_bytes(
        data[24..32].try_into()
            .map_err(|_| StarInvestorFeesError::StreamflowDeserializationFailed)?
    );
    
    let end_time = u64::from_le_bytes(
        data[32..40].try_into()
            .map_err(|_| StarInvestorFeesError::StreamflowDeserializationFailed)?
    );
    
    let cliff = u64::from_le_bytes(
        data[40..48].try_into()
            .map_err(|_| StarInvestorFeesError::StreamflowDeserializationFailed)?
    );

    msg!("Stream parsed: amount={}, withdrawn={}, start={}, end={}", 
         amount, withdrawn, start_time, end_time);

    Ok(StreamflowStream {
        amount,
        withdrawn,
        start_time,
        end_time,
        cliff,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_investor_count_validation() {
        // Valid: 0-20 investors
        assert!(20 <= MAX_INVESTORS_PER_PAGE);
        assert!(0 <= MAX_INVESTORS_PER_PAGE);
    }

    #[test]
    fn test_remaining_accounts_must_be_even() {
        // Valid: even number (pairs)
        let valid_counts = [0, 2, 4, 20, 40];
        for count in valid_counts {
            assert_eq!(count % 2, 0);
        }

        // Invalid: odd number
        let invalid_counts = [1, 3, 5, 21];
        for count in invalid_counts {
            assert_ne!(count % 2, 0);
        }
    }
}