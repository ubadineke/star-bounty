use anchor_lang::prelude::*;

/// Custom error codes for the Star Investor Fees program
///
/// Errors are organized by category for easier debugging and monitoring.
/// Each error includes a descriptive message that will be shown on-chain.
#[error_code]
pub enum StarInvestorFeesError {
    // ========== Position Initialization Errors (6000-6009) ==========
    #[msg("Base token fees detected - this position must only accrue quote fees")]
    BaseFeesDetected = 6000,

    #[msg("Invalid quote mint - does not match pool configuration")]
    InvalidQuoteMint,

    #[msg("Pool token order is invalid - cannot determine quote token")]
    InvalidPoolTokenOrder,

    #[msg("Position validation failed - not configured for quote-only accrual")]
    PositionValidationFailed,

    #[msg("Tick range invalid - lower tick must be less than upper tick")]
    InvalidTickRange,

    #[msg("Pool configuration is invalid or not initialized")]
    InvalidPoolConfig,

    #[msg("Position already exists for this vault")]
    PositionAlreadyExists,

    #[msg("Invalid tick spacing - must align with pool configuration")]
    InvalidTickSpacing,

    #[msg("Position is out of range - will not accrue fees")]
    PositionOutOfRange,

    #[msg("Position liquidity amount is invalid")]
    InvalidLiquidityAmount,

    // ========== Distribution Timing Errors (6010-6019) ==========
    #[msg("Distribution can only be called once per 24 hours - please wait")]
    DistributionTooEarly,

    #[msg("Day distribution already finalized - start a new day")]
    DayAlreadyFinalized,

    #[msg("Day distribution not ready for finalization - pages still pending")]
    DayNotReadyForFinalization,

    #[msg("Cannot start new distribution while current day is in progress")]
    DistributionInProgress,

    #[msg("Cannot finalize before first distribution of the day")]
    CannotFinalizeBeforeDistribution,

    #[msg("Distribution timestamp is invalid or in the future")]
    InvalidDistributionTimestamp,

    // ========== Configuration Errors (6020-6029) ==========
    #[msg("Investor fee share exceeds maximum allowed (10000 bps = 100%)")]
    InvalidFeeShareBps,

    #[msg("Invalid page number - must process pages sequentially starting from 0")]
    InvalidPageNumber,

    #[msg("Daily cap exceeded - cannot distribute more than configured limit")]
    DailyCapExceeded,

    #[msg("Payout amount below minimum threshold - will be carried as dust")]
    PayoutBelowMinimum,

    #[msg("Y0 total allocation must be greater than zero")]
    InvalidY0Allocation,

    #[msg("Minimum payout threshold is too high")]
    MinPayoutTooHigh,

    #[msg("Daily cap is lower than minimum payout - invalid configuration")]
    InvalidCapConfiguration,

    #[msg("Policy configuration is missing required parameters")]
    PolicyConfigInvalid,

    // ========== Data Integrity Errors (6030-6039) ==========
    #[msg("Total locked amount exceeds Y0 allocation - data integrity issue")]
    InvalidLockedTotal,

    #[msg("Zero total locked amount - no distribution needed")]
    ZeroTotalLocked,

    #[msg("Treasury has insufficient balance for distribution")]
    InsufficientTreasuryBalance,

    #[msg("Vault ID mismatch - account does not belong to this vault")]
    VaultIdMismatch,

    #[msg("Progress state corrupted - day start after current time")]
    ProgressStateCorrupted,

    #[msg("Inconsistent state - distributed amount exceeds claimed amount")]
    InconsistentDistributionState,

    #[msg("Investor data is corrupted or invalid")]
    InvalidInvestorData,

    // ========== Arithmetic Errors (6040-6049) ==========
    #[msg("Arithmetic overflow detected - amounts too large")]
    ArithmeticOverflow,

    #[msg("Arithmetic underflow detected - invalid subtraction")]
    ArithmeticUnderflow,

    #[msg("Division by zero attempted")]
    DivisionByZero,

    #[msg("Calculation result out of valid range")]
    CalculationOutOfRange,

    #[msg("Precision loss detected in calculation")]
    PrecisionLoss,

    // ========== External Program Errors (6050-6059) ==========
    #[msg("Invalid Streamflow stream account - cannot read vesting data")]
    InvalidStreamflowAccount,

    #[msg("Streamflow stream data deserialization failed - format mismatch")]
    StreamflowDeserializationFailed,

    #[msg("CP-AMM fee collection failed")]
    CpAmmFeeCollectionFailed,

    #[msg("CP-AMM position data invalid or corrupted")]
    CpAmmPositionInvalid,

    #[msg("CP-AMM program account is invalid")]
    InvalidCpAmmProgram,

    #[msg("Streamflow program account is invalid")]
    InvalidStreamflowProgram,

    #[msg("External program returned unexpected data")]
    ExternalProgramDataInvalid,

    // ========== Access Control Errors (6060-6069) ==========
    #[msg("Invalid authority - only the policy authority can perform this action")]
    InvalidAuthority,

    #[msg("Program is paused - operations temporarily disabled")]
    ProgramPaused,

    #[msg("Creator wallet does not match policy configuration")]
    InvalidCreator,

    #[msg("Signer is not authorized to perform this action")]
    UnauthorizedSigner,

    #[msg("Position owner PDA does not match expected derivation")]
    InvalidPositionOwner,

    #[msg("Cranker is not authorized for this operation")]
    InvalidCranker,

    // ========== Account Validation Errors (6070-6079) ==========
    #[msg("Treasury ATA does not match expected derivation")]
    InvalidTreasuryAta,

    #[msg("Invalid investor token account - must match quote mint")]
    InvalidInvestorAta,

    #[msg("Policy account not initialized")]
    PolicyNotInitialized,

    #[msg("Progress account not initialized")]
    ProgressNotInitialized,

    #[msg("Account owner is invalid")]
    InvalidAccountOwner,

    #[msg("Account data is corrupted")]
    CorruptedAccountData,

    #[msg("Account has insufficient rent exemption")]
    InsufficientRent,

    // ========== Pagination Errors (6080-6089) ==========
    #[msg("Too many investors in page - maximum 20 per page")]
    TooManyInvestorsPerPage,

    #[msg("Inconsistent investor data - stream/ATA count mismatch")]
    InconsistentInvestorData,

    #[msg("Page already processed - cannot reprocess")]
    PageAlreadyProcessed,

    #[msg("Invalid remaining accounts - must be even number (stream + ATA pairs)")]
    InvalidRemainingAccounts,

    #[msg("Pagination cursor is invalid")]
    InvalidPaginationCursor,

    #[msg("Cannot skip pages - must process sequentially")]
    CannotSkipPages,

    // ========== Fee Distribution Errors (6090-6099) ==========
    #[msg("No fees available to distribute")]
    NoFeesToDistribute,

    #[msg("Fee claim returned zero amount")]
    ZeroFeeClaim,

    #[msg("Dust accumulation overflow - amounts too large")]
    DustOverflow,

    #[msg("Distribution calculation error - invalid parameters")]
    DistributionCalculationError,

    #[msg("Fee split calculation failed")]
    FeeSplitCalculationFailed,

    #[msg("Token transfer failed")]
    TokenTransferFailed,

    #[msg("Claim instruction failed")]
    ClaimInstructionFailed,

    // ========== Tuktuk Integration Errors (6100-6109) ==========
    #[msg("Tuktuk task queue not found")]
    TuktukTaskQueueNotFound,

    #[msg("Tuktuk cron job creation failed")]
    TuktukCronCreationFailed,

    #[msg("Tuktuk task execution failed")]
    TuktukTaskFailed,

    #[msg("Tuktuk task queue is full")]
    TuktukQueueFull,

    #[msg("Tuktuk funding insufficient")]
    TuktukInsufficientFunding,
}

impl StarInvestorFeesError {
    /// Returns a user-friendly error message for each error code
    /// This helps users understand what went wrong without reading code
    pub fn to_user_message(&self) -> &str {
        match self {
            // Position Initialization
            Self::BaseFeesDetected => {
                "Position accrued base token fees. Only quote token fees are allowed."
            }
            Self::InvalidQuoteMint => "The quote token mint does not match the pool configuration.",
            Self::InvalidPoolTokenOrder => "Pool token configuration is incorrect.",
            Self::PositionValidationFailed => "Position configuration failed validation checks.",
            Self::InvalidTickRange => {
                "Tick range is invalid. Lower tick must be less than upper tick."
            }

            // Timing
            Self::DistributionTooEarly => "Please wait 24 hours before the next distribution.",
            Self::DayAlreadyFinalized => {
                "Today's distribution is complete. Please wait for tomorrow."
            }
            Self::DayNotReadyForFinalization => {
                "Cannot finalize yet - some pages still need processing."
            }

            // Configuration
            Self::InvalidFeeShareBps => {
                "Fee share must be between 0 and 10000 basis points (0-100%)."
            }
            Self::InvalidPageNumber => "Pages must be processed in order starting from 0.",
            Self::DailyCapExceeded => "Daily distribution limit reached.",
            Self::PayoutBelowMinimum => {
                "Payout too small. Will accumulate until minimum is reached."
            }

            // Access Control
            Self::ProgramPaused => "Distributions are temporarily paused. Please try again later.",
            Self::InvalidAuthority => "You don't have permission to perform this action.",
            Self::UnauthorizedSigner => "This wallet is not authorized.",

            // Data Integrity
            Self::ZeroTotalLocked => "No locked tokens found. Distribution skipped.",
            Self::InsufficientTreasuryBalance => "Not enough fees in treasury for distribution.",
            Self::InvalidLockedTotal => "Locked amount exceeds total allocation - data error.",

            // Arithmetic
            Self::ArithmeticOverflow => "Number too large for calculation.",
            Self::ArithmeticUnderflow => "Cannot subtract - result would be negative.",
            Self::DivisionByZero => "Cannot divide by zero.",

            // External Programs
            Self::InvalidStreamflowAccount => "Cannot read vesting data from Streamflow.",
            Self::StreamflowDeserializationFailed => "Streamflow data format error.",
            Self::CpAmmFeeCollectionFailed => "Failed to collect fees from pool.",

            // Pagination
            Self::TooManyInvestorsPerPage => "Too many investors in one page. Maximum is 20.",
            Self::InconsistentInvestorData => "Investor data is mismatched.",
            Self::PageAlreadyProcessed => "This page was already processed.",

            // Default
            _ => "An error occurred. Please check logs for details.",
        }
    }

    /// Returns whether this error is recoverable (retry might work)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::DistributionTooEarly
                | Self::PayoutBelowMinimum
                | Self::ZeroTotalLocked
                | Self::NoFeesToDistribute
                | Self::ProgramPaused
                | Self::ZeroFeeClaim
                | Self::DayNotReadyForFinalization
        )
    }

    /// Returns whether this error indicates a critical issue requiring immediate attention
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::BaseFeesDetected
                | Self::ProgressStateCorrupted
                | Self::InvalidLockedTotal
                | Self::ArithmeticOverflow
                | Self::ArithmeticUnderflow
                | Self::CorruptedAccountData
                | Self::InconsistentDistributionState
        )
    }

    /// Returns the error category for logging/monitoring
    pub fn category(&self) -> ErrorCategory {
        let code = *self as u32;

        match code {
            6000..=6009 => ErrorCategory::Initialization,
            6010..=6019 => ErrorCategory::Timing,
            6020..=6029 => ErrorCategory::Configuration,
            6030..=6039 => ErrorCategory::DataIntegrity,
            6040..=6049 => ErrorCategory::Arithmetic,
            6050..=6059 => ErrorCategory::ExternalProgram,
            6060..=6069 => ErrorCategory::AccessControl,
            6070..=6079 => ErrorCategory::AccountValidation,
            6080..=6089 => ErrorCategory::Pagination,
            6090..=6099 => ErrorCategory::Distribution,
            6100..=6109 => ErrorCategory::Tuktuk,
            _ => ErrorCategory::Other,
        }
    }

    /// Returns recommended action for this error
    pub fn recommended_action(&self) -> &str {
        match self {
            Self::DistributionTooEarly => "Wait until 24 hours have passed since last distribution",
            Self::ProgramPaused => "Contact protocol admin to unpause",
            Self::InvalidPageNumber => "Process pages sequentially starting from 0",
            Self::DailyCapExceeded => "Wait for next day or increase cap",
            Self::InsufficientTreasuryBalance => "Wait for fee accrual or trigger fee claim",
            Self::BaseFeesDetected => "Recreate position with correct tick range",
            Self::InvalidAuthority => "Use authorized wallet",
            Self::TooManyInvestorsPerPage => "Split into multiple pages of max 20 investors",
            Self::ZeroTotalLocked => "No action needed - normal when all tokens unlocked",
            _ => "Check documentation or contact support",
        }
    }
}

/// Error categories for monitoring and debugging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Initialization,
    Timing,
    Configuration,
    DataIntegrity,
    Arithmetic,
    ExternalProgram,
    AccessControl,
    AccountValidation,
    Pagination,
    Distribution,
    Tuktuk,
    Other,
}

impl ErrorCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Initialization => "initialization",
            Self::Timing => "timing",
            Self::Configuration => "configuration",
            Self::DataIntegrity => "data_integrity",
            Self::Arithmetic => "arithmetic",
            Self::ExternalProgram => "external_program",
            Self::AccessControl => "access_control",
            Self::AccountValidation => "account_validation",
            Self::Pagination => "pagination",
            Self::Distribution => "distribution",
            Self::Tuktuk => "tuktuk",
            Self::Other => "other",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Initialization => "Errors during position or account initialization",
            Self::Timing => "Errors related to distribution timing and scheduling",
            Self::Configuration => "Errors in policy or parameter configuration",
            Self::DataIntegrity => "Errors indicating data corruption or inconsistency",
            Self::Arithmetic => "Math operation errors (overflow, underflow, etc)",
            Self::ExternalProgram => "Errors from CP-AMM or Streamflow interactions",
            Self::AccessControl => "Permission and authorization errors",
            Self::AccountValidation => "Account structure or ownership errors",
            Self::Pagination => "Errors in multi-page processing",
            Self::Distribution => "Errors during fee distribution process",
            Self::Tuktuk => "Errors from Tuktuk automation integration",
            Self::Other => "Other uncategorized errors",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        assert_eq!(
            StarInvestorFeesError::BaseFeesDetected.category(),
            ErrorCategory::Initialization
        );
        assert_eq!(
            StarInvestorFeesError::DistributionTooEarly.category(),
            ErrorCategory::Timing
        );
        assert_eq!(
            StarInvestorFeesError::InvalidAuthority.category(),
            ErrorCategory::AccessControl
        );
        assert_eq!(
            StarInvestorFeesError::ArithmeticOverflow.category(),
            ErrorCategory::Arithmetic
        );
    }

    #[test]
    fn test_recoverable_errors() {
        assert!(StarInvestorFeesError::DistributionTooEarly.is_recoverable());
        assert!(StarInvestorFeesError::ProgramPaused.is_recoverable());
        assert!(StarInvestorFeesError::ZeroTotalLocked.is_recoverable());
        assert!(!StarInvestorFeesError::InvalidAuthority.is_recoverable());
    }

    #[test]
    fn test_critical_errors() {
        assert!(StarInvestorFeesError::BaseFeesDetected.is_critical());
        assert!(StarInvestorFeesError::ProgressStateCorrupted.is_critical());
        assert!(StarInvestorFeesError::ArithmeticOverflow.is_critical());
        assert!(!StarInvestorFeesError::DistributionTooEarly.is_critical());
        assert!(!StarInvestorFeesError::ProgramPaused.is_critical());
    }

    #[test]
    fn test_user_messages() {
        let error = StarInvestorFeesError::DistributionTooEarly;
        assert!(error.to_user_message().contains("24 hours"));

        let error = StarInvestorFeesError::InvalidFeeShareBps;
        assert!(error.to_user_message().contains("10000"));
    }

    #[test]
    fn test_error_code_ranges() {
        // Test that error codes are in expected ranges
        assert_eq!(StarInvestorFeesError::BaseFeesDetected as u32, 6000);
        assert_eq!(StarInvestorFeesError::DistributionTooEarly as u32, 6010);
        assert_eq!(StarInvestorFeesError::InvalidFeeShareBps as u32, 6020);
        assert_eq!(StarInvestorFeesError::InvalidLockedTotal as u32, 6030);
        assert_eq!(StarInvestorFeesError::ArithmeticOverflow as u32, 6040);
        assert_eq!(StarInvestorFeesError::InvalidStreamflowAccount as u32, 6050);
        assert_eq!(StarInvestorFeesError::InvalidAuthority as u32, 6060);
        assert_eq!(StarInvestorFeesError::InvalidTreasuryAta as u32, 6070);
        assert_eq!(StarInvestorFeesError::TooManyInvestorsPerPage as u32, 6080);
        assert_eq!(StarInvestorFeesError::NoFeesToDistribute as u32, 6090);
        assert_eq!(StarInvestorFeesError::TuktukTaskQueueNotFound as u32, 6100);
    }

    #[test]
    fn test_category_descriptions() {
        assert_eq!(ErrorCategory::Initialization.as_str(), "initialization");
        assert_eq!(ErrorCategory::Timing.as_str(), "timing");
        assert_eq!(ErrorCategory::Configuration.as_str(), "configuration");

        assert!(ErrorCategory::Initialization
            .description()
            .contains("position"));
        assert!(ErrorCategory::Timing.description().contains("timing"));
        assert!(ErrorCategory::Configuration
            .description()
            .contains("configuration"));
    }

    #[test]
    fn test_recommended_actions() {
        let error = StarInvestorFeesError::DistributionTooEarly;
        assert!(error.recommended_action().contains("24 hours"));

        let error = StarInvestorFeesError::InvalidAuthority;
        assert!(error.recommended_action().contains("authorized wallet"));

        let error = StarInvestorFeesError::TooManyInvestorsPerPage;
        assert!(error.recommended_action().contains("20"));
    }
}
// InvestorFeesError::BaseFeesDetected.is_recoverable());
//         assert!(!Star
