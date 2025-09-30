use crate::constants::*;
use crate::errors::StarInvestorFeesError;
use anchor_lang::prelude::*;

/// Safe math operations with overflow checks
pub mod safe_math {
    use super::*;

    /// Safe addition with overflow check
    pub fn add(a: u64, b: u64) -> Result<u64> {
        a.checked_add(b)
            .ok_or_else(|| StarInvestorFeesError::ArithmeticOverflow.into())
    }

    /// Safe subtraction with underflow check
    pub fn sub(a: u64, b: u64) -> Result<u64> {
        a.checked_sub(b)
            .ok_or_else(|| StarInvestorFeesError::ArithmeticUnderflow.into())
    }

    /// Safe multiplication with overflow check
    pub fn mul(a: u64, b: u64) -> Result<u64> {
        a.checked_mul(b)
            .ok_or_else(|| StarInvestorFeesError::ArithmeticOverflow.into())
    }

    /// Safe division with zero check
    pub fn div(a: u64, b: u64) -> Result<u64> {
        if b == 0 {
            return Err(StarInvestorFeesError::DivisionByZero.into());
        }
        Ok(a / b)
    }

    /// Safe multiplication using u128 for intermediate calculation
    pub fn mul128(a: u64, b: u64) -> Result<u128> {
        Ok((a as u128)
            .checked_mul(b as u128)
            .ok_or(StarInvestorFeesError::ArithmeticOverflow)?)
    }

    /// Safe division using u128 for intermediate calculation
    pub fn div128(a: u128, b: u128) -> Result<u64> {
        if b == 0 {
            return Err(StarInvestorFeesError::DivisionByZero.into());
        }
        let result = a
            .checked_div(b)
            .ok_or(StarInvestorFeesError::ArithmeticOverflow)?;
        u64::try_from(result).map_err(|_| StarInvestorFeesError::ArithmeticOverflow.into())
    }
}

/// Validation utilities
pub mod validation {
    use super::*;

    /// Validate vault ID matches expected
    // pub fn validate_vault_id(actual: &[u8; 32], expected: &[u8; 32]) -> Result<()> {
    //     require_eq!(
    //         actual,
    //         expected,
    //         StarInvestorFeesError::VaultIdMismatch
    //     );
    //     Ok(())
    // }

    /// Validate fee share is within valid range
    pub fn validate_fee_share_bps(bps: u16) -> Result<()> {
        require!(
            bps <= crate::constants::MAX_INVESTOR_FEE_SHARE_BPS,
            StarInvestorFeesError::InvalidFeeShareBps
        );
        Ok(())
    }

    /// Validate page number is sequential
    pub fn validate_page_number(actual: u16, expected: u16) -> Result<()> {
        require_eq!(actual, expected, StarInvestorFeesError::InvalidPageNumber);
        Ok(())
    }

    /// Validate timestamp is within valid range
    pub fn validate_timestamp(ts: i64) -> Result<()> {
        require!(ts > 0, StarInvestorFeesError::ProgressStateCorrupted);
        Ok(())
    }
}

/// PDA derivation utilities
pub mod pda {
    use super::*;
    // use crate::constants::*;

    /// Derive position owner PDA with bump verification
    pub fn derive_position_owner(
        program_id: &Pubkey,
        vault_id: &[u8; 32],
        expected_bump: u8,
    ) -> Result<Pubkey> {
        let seeds = &[
            VAULT_SEED,
            vault_id.as_ref(),
            POSITION_OWNER_SEED,
            &[expected_bump],
        ];

        let (pda, bump) = Pubkey::find_program_address(
            &[VAULT_SEED, vault_id.as_ref(), POSITION_OWNER_SEED],
            program_id,
        );

        require_eq!(
            bump,
            expected_bump,
            StarInvestorFeesError::InvalidPositionOwner
        );
        Ok(pda)
    }

    /// Get signer seeds for position owner PDA
    pub fn get_position_owner_signer_seeds<'a>(
        vault_id: &'a [u8; 32],
        bump: &'a [u8; 1],
    ) -> [&'a [u8]; 4] {
        [VAULT_SEED, vault_id.as_ref(), POSITION_OWNER_SEED, bump]
    }
}

/// Time utilities
pub mod time {
    use super::*;
    use crate::constants::SECONDS_PER_DAY;

    /// Check if enough time has passed since last distribution
    pub fn is_distribution_ready(last_ts: i64, current_ts: i64) -> bool {
        current_ts >= last_ts + SECONDS_PER_DAY
    }

    /// Get the start of the current day (midnight UTC)
    pub fn get_day_start(current_ts: i64) -> i64 {
        (current_ts / SECONDS_PER_DAY) * SECONDS_PER_DAY
    }

    /// Calculate time until next distribution
    pub fn time_until_next_distribution(last_ts: i64, current_ts: i64) -> i64 {
        let next_ts = last_ts + SECONDS_PER_DAY;
        next_ts.saturating_sub(current_ts).max(0)
    }
}

/// Fee calculation utilities
pub mod fee_calc {
    use super::*;
    use crate::constants::BPS_DENOMINATOR;

    /// Calculate f_locked(t) = locked_total / Y0
    /// Returns value in basis points (0-10000)
    pub fn calculate_f_locked(total_locked: u64, y0: u64) -> Result<u64> {
        if y0 == 0 {
            return Err(StarInvestorFeesError::InvalidLockedTotal.into());
        }

        let f_locked = safe_math::mul128(total_locked, BPS_DENOMINATOR)?;
        let result = safe_math::div128(f_locked, y0 as u128)?;

        Ok(result.min(BPS_DENOMINATOR))
    }

    /// Calculate eligible investor share based on locked percentage
    pub fn calculate_eligible_share(base_share_bps: u16, f_locked: u64) -> u16 {
        let scaled_f_locked = (f_locked as u32).min(BPS_DENOMINATOR as u32) as u16;
        base_share_bps.min(scaled_f_locked)
    }

    /// Calculate total investor fees from claimed amount
    pub fn calculate_investor_fee(claimed_amount: u64, share_bps: u16) -> Result<u64> {
        let fee = safe_math::mul128(claimed_amount, share_bps as u64)?;
        safe_math::div128(fee, BPS_DENOMINATOR as u128)
    }

    /// Calculate proportional payout for an investor
    pub fn calculate_proportional_payout(
        total_to_distribute: u64,
        investor_locked: u64,
        total_locked: u64,
    ) -> Result<u64> {
        if total_locked == 0 {
            return Ok(0);
        }

        let payout = safe_math::mul128(total_to_distribute, investor_locked)?;
        safe_math::div128(payout, total_locked as u128)
    }
}

/// Event emission helpers
pub mod events {
    use super::*;
    use crate::constants::*;

    /// Emit position initialized event with validation
    pub fn emit_position_initialized(
        vault_id: [u8; 32],
        position_owner: Pubkey,
        position: Pubkey,
        pool: Pubkey,
        quote_mint: Pubkey,
        lower_tick: i32,
        upper_tick: i32,
    ) -> Result<()> {
        let timestamp = Clock::get()?.unix_timestamp;
        emit!(HonoraryPositionInitialized {
            vault_id,
            position_owner,
            position,
            pool,
            quote_mint,
            lower_tick,
            upper_tick,
            timestamp,
        });
        Ok(())
    }

    /// Emit fees claimed event
    pub fn emit_fees_claimed(
        vault_id: [u8; 32],
        amount_claimed: u64,
        day_start: i64,
    ) -> Result<()> {
        let timestamp = Clock::get()?.unix_timestamp;
        emit!(QuoteFeesClaimed {
            vault_id,
            amount_claimed,
            timestamp,
            day_start,
        });
        Ok(())
    }

    /// Emit payout page event
    pub fn emit_payout_page(
        vault_id: [u8; 32],
        page_number: u16,
        investors_paid: u16,
        total_amount_paid: u64,
    ) -> Result<()> {
        let timestamp = Clock::get()?.unix_timestamp;
        emit!(InvestorPayoutPage {
            vault_id,
            page_number,
            investors_paid,
            total_amount_paid,
            timestamp,
        });
        Ok(())
    }

    /// Emit day closed event
    pub fn emit_day_closed(
        vault_id: [u8; 32],
        creator: Pubkey,
        amount_paid: u64,
        day_start: i64,
        day_end: i64,
    ) -> Result<()> {
        let timestamp = Clock::get()?.unix_timestamp;
        emit!(CreatorPayoutDayClosed {
            vault_id,
            creator,
            amount_paid,
            day_start,
            day_end,
            timestamp,
        });
        Ok(())
    }
}

/// Logging utilities for debugging
pub mod logging {
    use super::*;

    /// Log distribution summary
    pub fn log_distribution_summary(
        page: u16,
        investors_paid: u16,
        amount: u64,
        total_locked: u64,
    ) {
        msg!("=== Distribution Summary ===");
        msg!("Page: {}", page);
        msg!("Investors paid: {}", investors_paid);
        msg!("Amount distributed: {}", amount);
        msg!("Total locked: {}", total_locked);
        msg!("===========================");
    }

    /// Log fee claim details
    pub fn log_fee_claim(amount: u64, treasury_balance: u64) {
        msg!("=== Fee Claim ===");
        msg!("Amount claimed: {}", amount);
        msg!("Treasury balance: {}", treasury_balance);
        msg!("=================");
    }

    /// Log error with context
    pub fn log_error(error: &StarInvestorFeesError, context: &str) {
        msg!("ERROR [{}]: {:?}", context, error);
        msg!("Category: {:?}", error.category());
        msg!("User message: {}", error.to_user_message());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_math_add() {
        assert_eq!(safe_math::add(100, 200).unwrap(), 300);
        assert!(safe_math::add(u64::MAX, 1).is_err());
    }

    #[test]
    fn test_safe_math_sub() {
        assert_eq!(safe_math::sub(300, 100).unwrap(), 200);
        assert!(safe_math::sub(100, 200).is_err());
    }

    #[test]
    fn test_safe_math_mul() {
        assert_eq!(safe_math::mul(10, 20).unwrap(), 200);
        assert!(safe_math::mul(u64::MAX, 2).is_err());
    }

    #[test]
    fn test_safe_math_div() {
        assert_eq!(safe_math::div(100, 10).unwrap(), 10);
        assert!(safe_math::div(100, 0).is_err());
    }

    #[test]
    fn test_time_utils() {
        let last_ts = 1000;
        let current_ts = 87400; // > 24 hours later
        assert!(time::is_distribution_ready(last_ts, current_ts));

        let too_early = 1100;
        assert!(!time::is_distribution_ready(last_ts, too_early));
    }

    #[test]
    fn test_fee_calculations() {
        // Test f_locked calculation
        let total_locked = 500_000_000; // 500 tokens
        let y0 = 1_000_000_000; // 1000 tokens
        let f_locked = fee_calc::calculate_f_locked(total_locked, y0).unwrap();
        assert_eq!(f_locked, 5000); // 50% in basis points

        // Test investor fee calculation
        let claimed = 1_000_000; // 1 token
        let share_bps = 5000; // 50%
        let investor_fee = fee_calc::calculate_investor_fee(claimed, share_bps).unwrap();
        assert_eq!(investor_fee, 500_000); // 0.5 tokens

        // Test proportional payout
        let to_distribute = 1_000_000;
        let investor_locked = 250_000_000; // 250 tokens
        let total_locked = 1_000_000_000; // 1000 tokens
        let payout =
            fee_calc::calculate_proportional_payout(to_distribute, investor_locked, total_locked)
                .unwrap();
        assert_eq!(payout, 250_000); // 25% of distribution
    }
}
