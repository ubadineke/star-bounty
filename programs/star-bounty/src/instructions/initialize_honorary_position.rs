use crate::constants::*;
use crate::errors::StarInvestorFeesError;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{
    token_metadata_initialize, Mint, TokenAccount, TokenMetadataInitialize,
};

#[derive(Accounts)]
#[instruction(vault_id: [u8; 32])]
pub struct InitializeHonoraryPosition<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// PDA that owns the honorary position (receives the position NFT)
    /// Seeds: [VAULT_SEED, vault_id, POSITION_OWNER_SEED]
    /// CHECK: PDA derived and validated, receives position NFT
    #[account(
        seeds = [VAULT_SEED, vault_id.as_ref(), POSITION_OWNER_SEED],
        bump
    )]
    pub position_owner_pda: UncheckedAccount<'info>,

    /// Position NFT mint (created by Meteora)
    /// CHECK: testing
    #[account(mut, signer)]
    pub position_nft_mint: UncheckedAccount<'info>,

    /// Position NFT account (owned by our PDA)
    /// CHECK: Here and here
    #[account(mut)]
    pub position_nft_account: UncheckedAccount<'info>,

    /// Meteora DAMM v2 Pool
    /// CHECK: Meteora pool account
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,

    /// Position account (created by Meteora)
    /// CHECK: Meteora position account
    #[account(mut)]
    pub position: UncheckedAccount<'info>,

    /// Pool authority (Meteora PDA)
    /// CHECK: Meteora pool authority PDA
    pub pool_authority: UncheckedAccount<'info>,

    /// Quote mint (must be verified to match pool)
    pub quote_mint: InterfaceAccount<'info, Mint>,

    /// Base mint
    pub base_mint: InterfaceAccount<'info, Mint>,

    /// Pool's quote token vault
    #[account(
        constraint = pool_quote_vault.mint == quote_mint.key() @ StarInvestorFeesError::InvalidQuoteMint
    )]
    pub pool_quote_vault: InterfaceAccount<'info, TokenAccount>,

    /// Pool's base token vault
    #[account(
        constraint = pool_base_vault.mint == base_mint.key() @ StarInvestorFeesError::InvalidPoolTokenOrder
    )]
    pub pool_base_vault: InterfaceAccount<'info, TokenAccount>,

    /// Meteora program
    /// CHECK: Program ID validated
    #[account(
        constraint = meteora_program.key() == anchor_lang::solana_program::pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG")
            @ StarInvestorFeesError::InvalidCpAmmProgram
    )]
    pub meteora_program: UncheckedAccount<'info>,

    /// Token program (Token2022)
    pub token_program: Program<'info, Token2022>,

    /// System program
    pub system_program: Program<'info, System>,

    /// CHECK: Will check later
    pub event_authority: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<InitializeHonoraryPosition>, vault_id: [u8; 32]) -> Result<()> {
    msg!(
        "Initializing honorary position for vault_id: {:?}",
        ctx.accounts.payer.key().to_string()
    );

    // Validate vault_id is not all zeros
    require!(
        vault_id != [0u8; 32],
        StarInvestorFeesError::VaultIdMismatch
    );

    // Validate pool configuration and token order
    validate_pool_configuration(&ctx)?;

    // Log position details
    msg!(
        "Position owner PDA: {}",
        ctx.accounts.position_owner_pda.key()
    );
    msg!("Pool: {}", ctx.accounts.pool.key());
    msg!("Quote mint: {}", ctx.accounts.quote_mint.key());
    msg!("Base mint: {}", ctx.accounts.base_mint.key());
    msg!(
        "Position NFT mint: {}",
        ctx.accounts.position_nft_mint.key()
    );

    // Check mint to see if fee collection is quote only
    // Checking if 'collectFeeMode: 1' on the pool struct
    let collect_fee_mode = {
        let data = ctx.accounts.pool.data.borrow();
        data[484] // u8 value at offset 476 + 8(Discriminator)
    };

    require!(collect_fee_mode == 1, StarInvestorFeesError::InvalidFeeMode);

    msg!("collect_fee_mode = {}", collect_fee_mode);
    // Create position through Meteora via CPI
    create_meteora_position(&ctx, vault_id)?;

    // Emit event
    emit!(HonoraryPositionInitialized {
        vault_id,
        position_owner: ctx.accounts.position_owner_pda.key(),
        position: ctx.accounts.position.key(),
        pool: ctx.accounts.pool.key(),
        quote_mint: ctx.accounts.quote_mint.key(),
        lower_tick: 0, // Meteora positions don't have initial tick ranges
        upper_tick: 0, // These are set later via other instructions
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Honorary position initialized successfully");
    msg!(
        "Position NFT minted to PDA: {}",
        ctx.accounts.position_owner_pda.key()
    );

    Ok(())
}

/// Validate pool configuration and token order
fn validate_pool_configuration(ctx: &Context<InitializeHonoraryPosition>) -> Result<()> {
    // Verify that the pool vaults match the expected mints
    require!(
        ctx.accounts.pool_quote_vault.mint == ctx.accounts.quote_mint.key(),
        StarInvestorFeesError::InvalidQuoteMint
    );

    require!(
        ctx.accounts.pool_base_vault.mint == ctx.accounts.base_mint.key(),
        StarInvestorFeesError::InvalidPoolTokenOrder
    );

    // Verify that quote and base mints are different
    require!(
        ctx.accounts.quote_mint.key() != ctx.accounts.base_mint.key(),
        StarInvestorFeesError::InvalidPoolTokenOrder
    );

    msg!("Pool configuration validated");
    Ok(())
}

// / How the position will accrue quote-only fees:
// /
// / After position creation, we need to:
// / 1. Add liquidity in a specific price range (via separate instruction)
// / 2. Configure the range to only accrue fees in quote token
// / 3. This typically means positioning the range above or below current price
// /    depending on which token (0 or 1) is the quote token
// /
// / The actual fee accrual strategy depends on:
// / - Current pool price
// / - Which token is quote vs base
// / - Desired fee collection behavior
// /
// / For quote-only accrual:
// / - If quote = token0: position range should be ABOVE current price
// / - If quote = token1: position range should be BELOW current price
// /
// / This ensures swaps only generate fees in the quote token direction.

/// Create Meteora position via CPI
fn create_meteora_position(
    ctx: &Context<InitializeHonoraryPosition>,
    vault_id: [u8; 32],
) -> Result<()> {
    msg!("Creating Meteora position via CPI");

    // Get PDA signer seeds
    let position_owner_bump = ctx.bumps.position_owner_pda;
    let seeds = &[
        VAULT_SEED,
        vault_id.as_ref(),
        POSITION_OWNER_SEED,
        &[position_owner_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // TODO: Add Meteora instruction discriminator here
    let discriminator: [u8; 8] = [48, 215, 197, 153, 96, 203, 180, 133]; // discriminator from idl

    // Build instruction data
    let mut instruction_data = Vec::with_capacity(8);
    instruction_data.extend_from_slice(&discriminator);

    // Build account metas for Meteora's create_position instruction
    let account_metas = vec![
        AccountMeta::new_readonly(ctx.accounts.position_owner_pda.key(), false), // owner
        AccountMeta::new(ctx.accounts.position_nft_mint.key(), true), // position_nft_mint (signer)
        AccountMeta::new(ctx.accounts.position_nft_account.key(), false), // position_nft_account
        AccountMeta::new(ctx.accounts.pool.key(), false),             // pool
        AccountMeta::new(ctx.accounts.position.key(), false),         // position
        AccountMeta::new_readonly(ctx.accounts.pool_authority.key(), false), // pool_authority
        AccountMeta::new(ctx.accounts.payer.key(), true),             // payer (signer)
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false), // token_program
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false), // system_program
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
            ctx.accounts.position_owner_pda.to_account_info(),
            ctx.accounts.position_nft_mint.to_account_info(),
            ctx.accounts.position_nft_account.to_account_info(),
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.position.to_account_info(),
            ctx.accounts.pool_authority.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.event_authority.to_account_info(),
            ctx.accounts.meteora_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    msg!("Meteora position created successfully");
    msg!("Position: {}", ctx.accounts.position.key());
    msg!("NFT Mint: {}", ctx.accounts.position_nft_mint.key());

    Ok(())
}
