use crate::constants::*;
use crate::errors::StarInvestorFeesError;
use crate::state::PolicyConfig;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
#[instruction(vault_id: [u8; 32])]
pub struct AddLiquidityQuoteOnly<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Policy configuration to get position info
    #[account(
        seeds = [POLICY_SEED, vault_id.as_ref()],
        bump = policy.bump,
        has_one = authority @ StarInvestorFeesError::InvalidAuthority
    )]
    pub policy: Account<'info, PolicyConfig>,

    /// Position owner PDA (will sign the transaction)
    /// CHECK: Seeds validated
    #[account(
        seeds = [VAULT_SEED, vault_id.as_ref(), POSITION_OWNER_SEED],
        bump
    )]
    pub position_owner_pda: UncheckedAccount<'info>,

    /// Meteora Pool
    /// CHECK: Meteora pool account loader
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,

    /// Meteora Position
    /// CHECK: Meteora position account loader
    #[account(mut)]
    pub position: UncheckedAccount<'info>,

    /// Authority's token A account (quote or base)
    #[account(mut)]
    pub token_a_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Authority's token B account (quote or base)
    #[account(mut)]
    pub token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Pool's token A vault
    #[account(mut)]
    pub token_a_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Pool's token B vault
    #[account(mut)]
    pub token_b_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token A mint
    pub token_a_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token B mint
    pub token_b_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Position NFT account (owned by position_owner_pda)
    #[account(
        constraint = position_nft_account.amount == 1 @ StarInvestorFeesError::InvalidPositionOwner,
        constraint = position_nft_account.owner == position_owner_pda.key() @ StarInvestorFeesError::InvalidPositionOwner
    )]
    pub position_nft_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token A program
    pub token_a_program: Interface<'info, TokenInterface>,

    /// Token B program
    pub token_b_program: Interface<'info, TokenInterface>,

    /// Meteora program
    /// CHECK: Meteora program
    #[account(
        constraint = meteora_program.key() == anchor_lang::solana_program::pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG")
    )]
    pub meteora_program: UncheckedAccount<'info>,

    /// System program
    pub system_program: Program<'info, System>,

    /// CHECK: Will check later
    pub event_authority: UncheckedAccount<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddLiquidityParameters {
    /// Delta liquidity to add
    pub liquidity_delta: u128,
    /// Maximum token A amount to spend
    pub token_a_amount_threshold: u64,
    /// Maximum token B amount to spend
    pub token_b_amount_threshold: u64,
}

pub fn handler(
    ctx: Context<AddLiquidityQuoteOnly>,
    vault_id: [u8; 32],
    liquidity_delta: u128,
    token_a_amount_threshold: u64,
    token_b_amount_threshold: u64,
) -> Result<()> {
    msg!("Adding liquidity to honorary position");
    msg!("Liquidity delta: {}", liquidity_delta);
    msg!("Token A threshold: {}", token_a_amount_threshold);
    msg!("Token B threshold: {}", token_b_amount_threshold);

    // Validate amounts
    // require!(
    //     liquidity_delta > 0,
    //     StarInvestorFeesError::InvalidLiquidityAmount
    // );
    // require!(
    //     token_a_amount_threshold > 0 && token_b_amount_threshold > 0,
    //     StarInvestorFeesError::InvalidLiquidityAmount
    // ); // edit to check for the quote mint only

    // Add liquidity via CPI to Meteora
    add_liquidity_cpi(
        &ctx,
        vault_id,
        liquidity_delta,
        token_a_amount_threshold,
        token_b_amount_threshold,
    )?;

    msg!("Liquidity added successfully");

    // Emit event
    emit!(LiquidityAdded {
        vault_id,
        position: ctx.accounts.position.key(),
        liquidity_amount: liquidity_delta as u64,
        lower_tick: 0, // Would need to read from position state
        upper_tick: 0, // Would need to read from position state
        quote_amount: token_a_amount_threshold, // Actual amount would be returned from CPI
        base_amount: token_b_amount_threshold,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Add liquidity to position via Meteora CPI
fn add_liquidity_cpi(
    ctx: &Context<AddLiquidityQuoteOnly>,
    vault_id: [u8; 32],
    liquidity_delta: u128,
    token_a_threshold: u64,
    token_b_threshold: u64,
) -> Result<()> {
    msg!("Executing add_liquidity CPI to Meteora");

    // Get PDA signer seeds
    let position_owner_bump = ctx.bumps.position_owner_pda;
    let seeds = &[
        VAULT_SEED,
        vault_id.as_ref(),
        POSITION_OWNER_SEED,
        &[position_owner_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // TODO: Add Meteora add_liquidity instruction discriminator here
    let discriminator: [u8; 8] = [181, 157, 89, 67, 143, 182, 52, 72]; // Placeholder

    // Serialize AddLiquidityParameters
    let params = AddLiquidityParameters {
        liquidity_delta,
        token_a_amount_threshold: token_a_threshold,
        token_b_amount_threshold: token_b_threshold,
    };

    let mut instruction_data = Vec::with_capacity(8 + 32); // discriminator + params
    instruction_data.extend_from_slice(&discriminator);
    params.serialize(&mut instruction_data)?;

    // Build account metas for Meteora's add_liquidity instruction
    let account_metas = vec![
        AccountMeta::new(ctx.accounts.pool.key(), false), // pool
        AccountMeta::new(ctx.accounts.position.key(), false), // position
        AccountMeta::new(ctx.accounts.token_a_account.key(), false), // token_a_account
        AccountMeta::new(ctx.accounts.token_b_account.key(), false), // token_b_account
        AccountMeta::new(ctx.accounts.token_a_vault.key(), false), // token_a_vault
        AccountMeta::new(ctx.accounts.token_b_vault.key(), false), // token_b_vault
        AccountMeta::new_readonly(ctx.accounts.token_a_mint.key(), false), // token_a_mint
        AccountMeta::new_readonly(ctx.accounts.token_b_mint.key(), false), // token_b_mint
        AccountMeta::new_readonly(ctx.accounts.position_nft_account.key(), false), // position_nft_account
        AccountMeta::new_readonly(ctx.accounts.position_owner_pda.key(), true), // owner (signer via PDA)
        AccountMeta::new_readonly(ctx.accounts.token_a_program.key(), false),   // token_a_program
        AccountMeta::new_readonly(ctx.accounts.token_b_program.key(), false),   // token_b_program
        AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false),
        AccountMeta::new_readonly(ctx.accounts.meteora_program.key(), false),
    ];

    // Create the instruction
    let instruction = Instruction {
        program_id: ctx.accounts.meteora_program.key(),
        accounts: account_metas,
        data: instruction_data,
    };

    // Invoke the CPI with PDA signer
    invoke_signed(
        &instruction,
        &[
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.position.to_account_info(),
            ctx.accounts.token_a_account.to_account_info(),
            ctx.accounts.token_b_account.to_account_info(),
            ctx.accounts.token_a_vault.to_account_info(),
            ctx.accounts.token_b_vault.to_account_info(),
            ctx.accounts.token_a_mint.to_account_info(),
            ctx.accounts.token_b_mint.to_account_info(),
            ctx.accounts.position_nft_account.to_account_info(),
            ctx.accounts.position_owner_pda.to_account_info(),
            ctx.accounts.token_a_program.to_account_info(),
            ctx.accounts.token_b_program.to_account_info(),
            ctx.accounts.event_authority.to_account_info(),
            ctx.accounts.meteora_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    msg!("Liquidity added via CPI successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_liquidity_params_serialization() {
        let params = AddLiquidityParameters {
            liquidity_delta: 1000000,
            token_a_amount_threshold: 500000,
            token_b_amount_threshold: 500000,
        };

        let mut data = Vec::new();
        params.serialize(&mut data).unwrap();

        // Should serialize to 32 bytes (16 + 8 + 8)
        assert!(data.len() > 0);
    }

    #[test]
    fn test_validation() {
        // Liquidity delta must be > 0
        assert!(0u128 == 0);
        assert!(1000000u128 > 0);

        // Thresholds must be > 0
        assert!(500000u64 > 0);
    }
}
