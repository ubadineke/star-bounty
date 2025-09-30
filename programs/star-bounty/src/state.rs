use anchor_lang::prelude::*;

/// Policy configuration for fee distribution
#[account]
pub struct PolicyConfig {
    /// Vault ID for deterministic PDA derivation
    pub vault_id: [u8; 32],

    /// Authority that can update policy
    pub authority: Pubkey,

    /// Creator wallet to receive remainder fees
    pub creator: Pubkey,

    /// Base investor fee share in basis points (0-10000)
    pub investor_fee_share_bps: u16,

    /// Optional daily cap on distributions in lamports
    pub daily_cap_lamports: Option<u64>,

    /// Minimum payout threshold in lamports
    pub min_payout_lamports: u64,

    /// Y0 - Total investor allocation minted at TGE
    pub y0_total_allocation: u64,

    /// Quote mint for the pool
    pub quote_mint: Pubkey,

    /// Pool address
    pub pool: Pubkey,

    /// Honorary position address
    pub position: Pubkey,

    /// Emergency pause flag
    pub paused: bool,

    /// Bump for PDA derivation
    pub bump: u8,
}

impl PolicyConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // vault_id
        32 + // authority
        32 + // creator
        2 +  // investor_fee_share_bps
        9 +  // daily_cap_lamports (1 + 8)
        8 +  // min_payout_lamports
        8 +  // y0_total_allocation
        32 + // quote_mint
        32 + // pool
        32 + // position
        1 +  // paused
        1; // bump
}

/// Progress tracking for daily distribution
#[account]
pub struct DistributionProgress {
    /// Vault ID for deterministic PDA derivation
    pub vault_id: [u8; 32],

    /// Timestamp of last distribution start
    pub last_distribution_ts: i64,

    /// Current day start timestamp
    pub current_day_start: i64,

    /// Total amount claimed from pool this day
    pub daily_claimed_amount: u64,

    /// Total distributed to investors this day
    pub daily_distributed_to_investors: u64,

    /// Total distributed to creator this day
    pub daily_distributed_to_creator: u64,

    /// Current page being processed
    pub current_page: u16,

    /// Total pages to process this day
    pub total_pages: u16,

    /// Carry-over dust from previous day
    pub carry_over_dust: u64,

    /// Flag indicating if day is finalized
    pub day_finalized: bool,

    /// Bump for PDA derivation
    pub bump: u8,
}

impl DistributionProgress {
    pub const LEN: usize = 8 +  // discriminator
        32 + // vault_id
        8 +  // last_distribution_ts
        8 +  // current_day_start
        8 +  // daily_claimed_amount
        8 +  // daily_distributed_to_investors
        8 +  // daily_distributed_to_creator
        2 +  // current_page
        2 +  // total_pages
        8 +  // carry_over_dust
        1 +  // day_finalized
        1; // bump

    pub fn reset_for_new_day(&mut self, timestamp: i64, claimed_amount: u64) {
        self.current_day_start = timestamp;
        self.daily_claimed_amount = claimed_amount;
        self.daily_distributed_to_investors = 0;
        self.daily_distributed_to_creator = 0;
        self.current_page = 0;
        self.total_pages = 0;
        self.day_finalized = false;
    }

    pub fn is_new_day_ready(&self, current_ts: i64) -> bool {
        current_ts >= self.last_distribution_ts + crate::constants::SECONDS_PER_DAY
    }
}

/// Investor input for distribution (passed as remaining accounts)
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InvestorInput {
    /// Investor's token account to receive fees
    pub investor_ata: Pubkey,

    /// Streamflow stream account for this investor
    pub streamflow_stream: Pubkey,
}

/// Streamflow stream data structure (simplified - only what we need)
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct StreamflowStream {
    pub amount: u64,
    pub withdrawn: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub cliff: u64,
}

impl StreamflowStream {
    /// Calculate currently locked amount at given timestamp
    pub fn calculate_locked_amount(&self, current_time: u64) -> u64 {
        if current_time < self.start_time {
            return self.amount;
        }

        if current_time >= self.end_time {
            return 0;
        }

        let elapsed = current_time.saturating_sub(self.start_time);
        let total_duration = self.end_time.saturating_sub(self.start_time);

        if elapsed < self.cliff {
            return self.amount;
        }

        let unlocked = self
            .amount
            .checked_mul(elapsed)
            .and_then(|v| v.checked_div(total_duration))
            .unwrap_or(0);

        self.amount
            .saturating_sub(unlocked)
            .saturating_sub(self.withdrawn)
    }
}
