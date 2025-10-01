use anchor_lang::prelude::*;

// pub mod constants;
pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("2WYpJx4kYNRKpCm4wPPNZjWFJvpqU2KCCxa88xcHuKdL");

#[program]
pub mod star_bounty {
    use super::*;

    /// Initialize the honorary DAMM v2 position for quote-only fee accrual
    ///
    /// This creates a special liquidity position owned by the program that only
    /// accrues fees in the quote token (not the base token).
    ///
    /// # Arguments
    /// * `vault_id` - Unique identifier for this vault (32 bytes)
    /// * `lower_tick_index` - Lower bound of the price range
    /// * `upper_tick_index` - Upper bound of the price range
    pub fn initialize_honorary_position(
        ctx: Context<InitializeHonoraryPosition>,
        vault_id: [u8; 32],
    ) -> Result<()> {
        // instructions::initialize_honorary_position::handler(ctx, vault_id, lower_tick_index, upper_tick_index)
        instructions::initialize_honorary_position::handler(ctx, vault_id)
    }

    /// Initialize the policy configuration
    ///
    /// Sets up the fee distribution parameters and configuration.
    ///
    /// # Arguments
    /// * `vault_id` - Unique identifier for this vault
    /// * `investor_fee_share_bps` - Investor fee share in basis points (0-10000)
    /// * `daily_cap_lamports` - Optional daily distribution cap
    /// * `min_payout_lamports` - Minimum payout threshold
    /// * `y0_total_allocation` - Total investor allocation at TGE
    pub fn initialize_policy(
        ctx: Context<InitializePolicy>,
        vault_id: [u8; 32],
        investor_fee_share_bps: u16,
        daily_cap_lamports: Option<u64>,
        min_payout_lamports: u64,
        y0_total_allocation: u64,
    ) -> Result<()> {
        instructions::initialize_policy::handler(
            ctx,
            vault_id,
            investor_fee_share_bps,
            daily_cap_lamports,
            min_payout_lamports,
            y0_total_allocation,
        )
    }

    pub fn add_liquidity_quote_only(
        ctx: Context<AddLiquidityQuoteOnly>,
        vault_id: [u8; 32],
        liquidity_delta: u128,
        token_a_amount_threshold: u64,
        token_b_amount_threshold: u64,
    ) -> Result<()> {
        instructions::add_liquidity_quote_only::handler(
            ctx,
            vault_id,
            liquidity_delta,
            token_a_amount_threshold,
            token_b_amount_threshold,
        )
    }
    // / Main distribution crank - claims fees and distributes to investors
    // /
    // / This is the core function that should be called once per 24 hours.
    // / It can be called multiple times with different page_numbers to process
    // / all investors in a paginated manner.
    // /
    // / # Arguments
    // / * `vault_id` - Unique identifier for this vault
    // / * `page_number` - Current page being processed (0-indexed)
    // /
    // / # Remaining Accounts
    // / For each investor on this page (up to 20):
    // / - Streamflow stream account (read-only)
    // / - Investor quote token account (writable)
    // pub fn distribute_fees(
    //     ctx: Context<DistributeFees>,
    //     vault_id: [u8; 32],
    //     page_number: u16,
    // ) -> Result<()> {
    //     instructions::distribute_fees::handler(ctx, vault_id, page_number)
    // }

    // / Finalize the day by sending remainder to creator
    // /
    // / Should be called after all pages have been processed.
    // / Sends any remaining fees to the project creator.
    // /
    // / # Arguments
    // / * `vault_id` - Unique identifier for this vault
    // pub fn finalize_day_distribution(
    //     ctx: Context<FinalizeDayDistribution>,
    //     vault_id: [u8; 32],
    // ) -> Result<()> {
    //     instructions::finalize_day_distribution::handler(ctx, vault_id)
    // }

    // / Update policy configuration (admin only)
    // /
    // / Allows the authority to update distribution parameters.
    // /
    // / # Arguments
    // / * `vault_id` - Unique identifier for this vault
    // / * `investor_fee_share_bps` - New investor fee share (optional)
    // / * `daily_cap_lamports` - New daily cap (optional)
    // / * `min_payout_lamports` - New minimum payout (optional)
    // pub fn update_policy(
    //     ctx: Context<UpdatePolicy>,
    //     vault_id: [u8; 32],
    //     investor_fee_share_bps: Option<u16>,
    //     daily_cap_lamports: Option<Option<u64>>,
    //     min_payout_lamports: Option<u64>,
    // ) -> Result<()> {
    //     instructions::update_policy::handler(
    //         ctx,
    //         vault_id,
    //         investor_fee_share_bps,
    //         daily_cap_lamports,
    //         min_payout_lamports,
    //     )
    // }

    // / Emergency pause mechanism
    // /
    // / Allows the authority to pause/unpause distributions in case of emergency.
    // /
    // / # Arguments
    // / * `vault_id` - Unique identifier for this vault
    // / * `paused` - Whether to pause (true) or unpause (false)
    // pub fn set_pause_state(
    //     ctx: Context<SetPauseState>,
    //     vault_id: [u8; 32],
    //     paused: bool,
    // ) -> Result<()> {
    //     instructions::set_pause_state::handler(ctx, vault_id, paused)
    // }
}
