use anchor_lang::prelude::*;

/// PDA Seeds for deterministic address derivation
pub const VAULT_SEED: &[u8] = b"vault";
pub const POSITION_OWNER_SEED: &[u8] = b"investor_fee_pos_owner";
pub const POLICY_SEED: &[u8] = b"policy";
pub const PROGRESS_SEED: &[u8] = b"progress";
pub const TREASURY_SEED: &[u8] = b"treasury";

/// Time constants
pub const SECONDS_PER_DAY: i64 = 86_400; // 24 hours in seconds
pub const SECONDS_PER_HOUR: i64 = 3_600;

/// Math constants
pub const BPS_DENOMINATOR: u64 = 10_000; // 100% = 10,000 basis points
pub const MAX_INVESTOR_FEE_SHARE_BPS: u16 = 10_000; // 100% maximum
pub const PERCENTAGE_MULTIPLIER: u64 = 100;

/// Pagination constants
pub const MAX_INVESTORS_PER_PAGE: usize = 20; // Max investors per distribution page
pub const MAX_PAGES_PER_DAY: u16 = 1000; // Safety limit on pagination

/// Validation constants
pub const MIN_Y0_ALLOCATION: u64 = 1; // Minimum Y0 allocation
pub const MAX_DAILY_CAP: u64 = u64::MAX; // Theoretical maximum
pub const DEFAULT_MIN_PAYOUT: u64 = 10_000; // 0.01 tokens (6 decimals)

/// Program version and metadata
pub const PROGRAM_VERSION: &str = "1.0.0";
pub const PROGRAM_NAME: &str = "Star Investor Fees";

/// External program IDs
pub const METEORA_PROGRAM_ID: Pubkey =
    anchor_lang::solana_program::pubkey!("24Uqj9JCLxUeoC3hGfh5W3s9FM9uCHDS2SG3LYwBpyTi");
// pub const STREAMFLOW_PROGRAM_ID: Pubkey = anchor_lang::solana_program::pubkey!("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUTNYXr6vk");
pub const TUKTUK_PROGRAM_ID: Pubkey =
    anchor_lang::solana_program::pubkey!("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA");

/// Common token mints (for reference)
pub const USDC_MINT: Pubkey =
    anchor_lang::solana_program::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const SOL_MINT: Pubkey =
    anchor_lang::solana_program::pubkey!("So11111111111111111111111111111111111111112");

/// Tick constants (for concentrated liquidity)
pub const MIN_TICK: i32 = -443_636;
pub const MAX_TICK: i32 = 443_636;
pub const DEFAULT_TICK_SPACING: i32 = 60;

/// Liquidity constants  
pub const MIN_LIQUIDITY: u64 = 1000; // Minimum liquidity amount
pub const DUST_THRESHOLD: u64 = 100; // Below this amount is considered dust

/// Event discriminators and sizes
pub const MAX_EVENT_DATA_SIZE: usize = 1024;
pub const MAX_LOG_MESSAGE_SIZE: usize = 512;

// ============================================================================
// EVENT DEFINITIONS
// ============================================================================

/// Emitted when an honorary position is initialized
#[event]
pub struct HonoraryPositionInitialized {
    pub vault_id: [u8; 32],
    pub position_owner: Pubkey,
    pub position: Pubkey,
    pub pool: Pubkey,
    pub quote_mint: Pubkey,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub timestamp: i64,
}

/// Emitted when fees are claimed from the pool
#[event]
pub struct QuoteFeesClaimed {
    pub vault_id: [u8; 32],
    pub amount_claimed: u64,
    pub timestamp: i64,
    pub day_start: i64,
}

/// Emitted for each page of investor payouts
#[event]
pub struct InvestorPayoutPage {
    pub vault_id: [u8; 32],
    pub page_number: u16,
    pub investors_paid: u16,
    pub total_amount_paid: u64,
    pub timestamp: i64,
}

/// Emitted when a day's distribution is finalized
#[event]
pub struct CreatorPayoutDayClosed {
    pub vault_id: [u8; 32],
    pub creator: Pubkey,
    pub amount_paid: u64,
    pub day_start: i64,
    pub day_end: i64,
    pub timestamp: i64,
}

/// Emitted when policy configuration is updated
#[event]
pub struct PolicyUpdated {
    pub vault_id: [u8; 32],
    pub investor_fee_share_bps: u16,
    pub daily_cap_lamports: Option<u64>,
    pub min_payout_lamports: u64,
    pub timestamp: i64,
}

/// Emitted when pause state changes
#[event]
pub struct PauseStateChanged {
    pub vault_id: [u8; 32],
    pub paused: bool,
    pub timestamp: i64,
}

/// Emitted when liquidity is added to position
#[event]
pub struct LiquidityAdded {
    pub vault_id: [u8; 32],
    pub position: Pubkey,
    pub liquidity_amount: u64,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub quote_amount: u64,
    pub base_amount: u64,
    pub timestamp: i64,
}

/// Emitted for dust accumulation tracking
#[event]
pub struct DustAccumulated {
    pub vault_id: [u8; 32],
    pub amount: u64,
    pub total_dust: u64,
    pub timestamp: i64,
}

/// Emitted for emergency actions
#[event]
pub struct EmergencyAction {
    pub vault_id: [u8; 32],
    pub action: String,
    pub authority: Pubkey,
    pub timestamp: i64,
}

// ============================================================================
// HELPER MACROS
// ============================================================================

/// Macro for creating deterministic PDAs
#[macro_export]
macro_rules! find_pda {
    ($seeds:expr, $program_id:expr) => {
        Pubkey::find_program_address($seeds, $program_id)
    };
}

/// Macro for validating basis points
#[macro_export]
macro_rules! validate_bps {
    ($bps:expr) => {
        require!($bps <= MAX_INVESTOR_FEE_SHARE_BPS, "Invalid basis points")
    };
}

/// Macro for safe arithmetic operations
#[macro_export]
macro_rules! safe_add {
    ($a:expr, $b:expr) => {
        $a.checked_add($b)
            .ok_or(crate::errors::StarInvestorFeesError::ArithmeticOverflow)
    };
}

#[macro_export]
macro_rules! safe_sub {
    ($a:expr, $b:expr) => {
        $a.checked_sub($b)
            .ok_or(crate::errors::StarInvestorFeesError::ArithmeticUnderflow)
    };
}

#[macro_export]
macro_rules! safe_mul {
    ($a:expr, $b:expr) => {
        $a.checked_mul($b)
            .ok_or(crate::errors::StarInvestorFeesError::ArithmeticOverflow)
    };
}

#[macro_export]
macro_rules! safe_div {
    ($a:expr, $b:expr) => {{
        if $b == 0 {
            Err(crate::errors::StarInvestorFeesError::DivisionByZero)
        } else {
            Ok($a / $b)
        }
    }};
}

// ============================================================================
// CONFIGURATION PRESETS
// ============================================================================

/// Conservative configuration preset
pub struct ConservativeConfig;
impl ConservativeConfig {
    pub const INVESTOR_FEE_SHARE_BPS: u16 = 3000; // 30%
    pub const DAILY_CAP_FACTOR: u64 = 100; // Cap at 100x min payout
    pub const MIN_PAYOUT_LAMPORTS: u64 = 100_000; // 0.1 tokens
}

/// Moderate configuration preset  
pub struct ModerateConfig;
impl ModerateConfig {
    pub const INVESTOR_FEE_SHARE_BPS: u16 = 5000; // 50%
    pub const DAILY_CAP_FACTOR: u64 = 200; // Cap at 200x min payout
    pub const MIN_PAYOUT_LAMPORTS: u64 = 50_000; // 0.05 tokens
}

/// Aggressive configuration preset
pub struct AggressiveConfig;
impl AggressiveConfig {
    pub const INVESTOR_FEE_SHARE_BPS: u16 = 7500; // 75%
    pub const DAILY_CAP_FACTOR: u64 = 500; // Cap at 500x min payout
    pub const MIN_PAYOUT_LAMPORTS: u64 = 10_000; // 0.01 tokens
}

// ============================================================================
// UTILITY CONSTANTS
// ============================================================================

/// Standard token decimals
pub mod decimals {
    pub const USDC: u8 = 6;
    pub const SOL: u8 = 9;
    pub const ETH: u8 = 8;
    pub const BTC: u8 = 8;
    pub const DEFAULT: u8 = 6;
}

/// Common amounts in lamports (assuming 6 decimals)
pub mod amounts {
    pub const ONE_TOKEN: u64 = 1_000_000; // 1.0 token with 6 decimals
    pub const TEN_TOKENS: u64 = 10_000_000; // 10.0 tokens
    pub const HUNDRED_TOKENS: u64 = 100_000_000; // 100.0 tokens
    pub const THOUSAND_TOKENS: u64 = 1_000_000_000; // 1,000.0 tokens
}

/// Time periods in seconds
pub mod timeframes {
    pub const ONE_MINUTE: i64 = 60;
    pub const FIVE_MINUTES: i64 = 300;
    pub const ONE_HOUR: i64 = 3_600;
    pub const ONE_DAY: i64 = 86_400;
    pub const ONE_WEEK: i64 = 604_800;
    pub const ONE_MONTH: i64 = 2_592_000; // 30 days
}

// ============================================================================
// ERROR MESSAGES
// ============================================================================

pub mod error_messages {
    pub const INSUFFICIENT_BALANCE: &str = "Insufficient balance for operation";
    pub const INVALID_AUTHORITY: &str = "Invalid authority for this operation";
    pub const PROGRAM_PAUSED: &str = "Program is currently paused";
    pub const DISTRIBUTION_TOO_EARLY: &str = "Must wait 24 hours between distributions";
    pub const NO_FEES_TO_DISTRIBUTE: &str = "No fees available for distribution";
    pub const INVALID_PAGE_NUMBER: &str = "Invalid page number - process sequentially";
    pub const DAILY_CAP_EXCEEDED: &str = "Daily distribution cap has been exceeded";
    pub const BASE_FEES_DETECTED: &str = "Base fees detected - only quote fees allowed";
    pub const ZERO_LOCKED_AMOUNT: &str = "No locked tokens found for distribution";
}

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

/// Validate a vault ID is not all zeros
pub fn validate_vault_id(vault_id: &[u8; 32]) -> bool {
    *vault_id != [0u8; 32]
}

/// Validate basis points are within range
pub fn validate_basis_points(bps: u16) -> bool {
    bps <= MAX_INVESTOR_FEE_SHARE_BPS
}

/// Validate timestamp is reasonable
pub fn validate_timestamp(ts: i64) -> bool {
    ts > 0 && ts < i64::MAX
}

/// Validate tick range
pub fn validate_tick_range(lower: i32, upper: i32) -> bool {
    lower >= MIN_TICK
        && upper <= MAX_TICK
        && lower < upper
        && lower % DEFAULT_TICK_SPACING == 0
        && upper % DEFAULT_TICK_SPACING == 0
}

/// Calculate percentage from basis points
pub fn bps_to_percentage(bps: u16) -> f64 {
    bps as f64 / 100.0
}

/// Convert percentage to basis points
pub fn percentage_to_bps(percentage: f64) -> u16 {
    (percentage * 100.0) as u16
}

/// Get current Unix timestamp
pub fn current_timestamp() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(BPS_DENOMINATOR, 10_000);
        assert_eq!(MAX_INVESTOR_FEE_SHARE_BPS, 10_000);
        assert_eq!(SECONDS_PER_DAY, 86_400);
        assert_eq!(MAX_INVESTORS_PER_PAGE, 20);
    }

    #[test]
    fn test_validation_helpers() {
        // Valid vault ID
        let valid_vault_id = [1u8; 32];
        assert!(validate_vault_id(&valid_vault_id));

        // Invalid vault ID (all zeros)
        let invalid_vault_id = [0u8; 32];
        assert!(!validate_vault_id(&invalid_vault_id));

        // Valid basis points
        assert!(validate_basis_points(5000));
        assert!(validate_basis_points(0));
        assert!(validate_basis_points(10000));

        // Invalid basis points
        assert!(!validate_basis_points(10001));

        // Valid tick range
        assert!(validate_tick_range(-120, 180));

        // Invalid tick range
        assert!(!validate_tick_range(180, -120)); // upper < lower
        assert!(!validate_tick_range(-119, 180)); // not aligned to spacing
    }

    #[test]
    fn test_percentage_conversion() {
        assert_eq!(bps_to_percentage(5000), 50.0);
        assert_eq!(bps_to_percentage(10000), 100.0);
        assert_eq!(bps_to_percentage(0), 0.0);

        assert_eq!(percentage_to_bps(50.0), 5000);
        assert_eq!(percentage_to_bps(100.0), 10000);
        assert_eq!(percentage_to_bps(0.0), 0);
    }

    #[test]
    fn test_config_presets() {
        assert!(ConservativeConfig::INVESTOR_FEE_SHARE_BPS <= MAX_INVESTOR_FEE_SHARE_BPS);
        assert!(ModerateConfig::INVESTOR_FEE_SHARE_BPS <= MAX_INVESTOR_FEE_SHARE_BPS);
        assert!(AggressiveConfig::INVESTOR_FEE_SHARE_BPS <= MAX_INVESTOR_FEE_SHARE_BPS);

        // Conservative should be less than aggressive
        assert!(
            ConservativeConfig::INVESTOR_FEE_SHARE_BPS < AggressiveConfig::INVESTOR_FEE_SHARE_BPS
        );
    }

    #[test]
    fn test_amount_constants() {
        assert_eq!(amounts::ONE_TOKEN, 1_000_000);
        assert_eq!(amounts::TEN_TOKENS, 10_000_000);
        assert_eq!(amounts::HUNDRED_TOKENS, 100_000_000);
    }

    #[test]
    fn test_timeframe_constants() {
        assert_eq!(timeframes::ONE_DAY, SECONDS_PER_DAY);
        assert_eq!(timeframes::ONE_HOUR, SECONDS_PER_HOUR);
        assert_eq!(timeframes::ONE_WEEK, 7 * SECONDS_PER_DAY);
    }

    #[test]
    fn test_tick_constants() {
        assert!(MIN_TICK < 0);
        assert!(MAX_TICK > 0);
        assert_eq!(MIN_TICK, -443_636);
        assert_eq!(MAX_TICK, 443_636);
    }
}
