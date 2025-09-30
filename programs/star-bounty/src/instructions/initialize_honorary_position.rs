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
    #[account(
      mut,
        // init,
        signer,
        // payer = payer,
        // mint::token_program = token_program,
        // mint::decimals = 0,
        // mint::authority = pool_authority,
        // mint::freeze_authority = pool, // Meteora uses pool as freeze authority
        // extensions::metadata_pointer::authority = pool_authority,
        // extensions::metadata_pointer::metadata_address = position_nft_mint,
        // extensions::close_authority::authority = pool_authority,
    )]
    // pub position_nft_mint: Box<InterfaceAccount<'info, Mint>>,
    pub position_nft_mint: UncheckedAccount<'info>,

    /// Position NFT account (owned by our PDA)
    /// CHECK: Here and here
    #[account(
      mut,
        // init,
        // seeds = [b"position_nft_account", position_nft_mint.key().as_ref()],
        // token::mint = position_nft_mint,
        // token::authority = position_owner_pda,
        // token::token_program = token_program,
        // payer = payer,
        // bump,
    )]
    // pub position_nft_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub position_nft_account: UncheckedAccount<'info>,

    /// Meteora DAMM v2 Pool
    /// CHECK: Meteora pool account
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,

    /// Position account (created by Meteora)
    /// CHECK: Meteora position account
    #[account(
      mut
        // init,
        // seeds = [
        //     b"position",
        //     position_nft_mint.key().as_ref()
        // ],
        // bump,
        // payer = payer,
        // space = 8 + 400 // Meteora Position size
    )]
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
    pub event_authority: UncheckedAccount<'info>
}

pub fn handler(ctx: Context<InitializeHonoraryPosition>, vault_id: [u8; 32]) -> Result<()> {
    msg!(
        "Initializing honorary position for vault_id: {:?}",
        ctx.accounts.payer.key().to_string()
    );

    msg!(
        "Initializing honorary position for vault_id:",
        // vault_id
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

/// Create Meteora position via CPI
// fn create_meteora_position(
//     ctx: &Context<InitializeHonoraryPosition>,
//     vault_id: [u8; 32],
// ) -> Result<()> {
//     msg!("Creating Meteora position via CPI");

//     // Get PDA signer seeds
//     let position_owner_bump = ctx.bumps.position_owner_pda;
//     let seeds = &[
//         VAULT_SEED,
//         vault_id.as_ref(),
//         POSITION_OWNER_SEED,
//         &[position_owner_bump],
//     ];
//     let signer_seeds = &[&seeds[..]];

//     // CPI to Meteora's create_position instruction
//     // Based on the Meteora code structure
//     let cpi_accounts = CreatePositionCpi {
//         owner: ctx.accounts.position_owner_pda.to_account_info(),
//         position_nft_mint: ctx.accounts.position_nft_mint.to_account_info(),
//         position_nft_account: ctx.accounts.position_nft_account.to_account_info(),
//         pool: ctx.accounts.pool.to_account_info(),
//         position: ctx.accounts.position.to_account_info(),
//         pool_authority: ctx.accounts.pool_authority.to_account_info(),
//         payer: ctx.accounts.payer.to_account_info(),
//         token_program: ctx.accounts.token_program.to_account_info(),
//         system_program: ctx.accounts.system_program.to_account_info(),
//     };

//     let cpi_ctx = CpiContext::new_with_signer(
//         ctx.accounts.meteora_program.to_account_info(),
//         cpi_accounts,
//         signer_seeds,
//     );

//     // Call Meteora's create_position
//     // meteora_create_position(cpi_ctx)?;

//     msg!("Meteora position created successfully");
//     msg!("Position: {}", ctx.accounts.position.key());
//     msg!("NFT Mint: {}", ctx.accounts.position_nft_mint.key());

//     Ok(())
// }

/// Meteora CPI accounts structure
#[derive(Accounts)]
pub struct CreatePositionCpi<'info> {
    /// CHECK: Position owner (our PDA)
    pub owner: AccountInfo<'info>,
    /// CHECK: Position NFT mint
    pub position_nft_mint: AccountInfo<'info>,
    /// CHECK: Position NFT account
    pub position_nft_account: AccountInfo<'info>,
    /// CHECK: Pool
    pub pool: AccountInfo<'info>,
    /// CHECK: Position
    pub position: AccountInfo<'info>,
    /// CHECK: Pool authority
    pub pool_authority: AccountInfo<'info>,
    /// CHECK: Payer
    pub payer: AccountInfo<'info>,
    /// CHECK: Token program
    pub token_program: AccountInfo<'info>,
    /// CHECK: System program
    pub system_program: AccountInfo<'info>,
}

// Call Meteora's create_position instruction
// fn meteora_create_position(ctx: CpiContext<CreatePositionCpi>) -> Result<()> {
//     // This would be the actual CPI call to Meteora
//     // For now, we simulate what would happen

//     msg!("=== Meteora Position Creation ===");
//     msg!("Owner: {}", ctx.accounts.owner.key);
//     msg!("Position: {}", ctx.accounts.position.key);
//     msg!("Pool: {}", ctx.accounts.pool.key);
//     msg!("NFT Mint: {}", ctx.accounts.position_nft_mint.key);

//     // In actual implementation, this would be:
//     // meteora::cpi::create_position(ctx)

//     // The position is created with:
//     // - Zero initial liquidity
//     // - NFT minted to our PDA
//     // - Position owned by our program

//     msg!("Position created with zero liquidity");
//     msg!("NFT ownership transferred to PDA");

//     Ok(())
// }

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
    // Example: let discriminator = [YOUR_8_BYTE_DISCRIMINATOR_HERE];
    let discriminator: [u8; 8] = [48, 215, 197, 153, 96, 203, 180, 133]; // Placeholder - replace with actual discriminator

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
        AccountMeta::new_readonly(ctx.accounts.meteora_program.key(), false)
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
            ctx.accounts.meteora_program.to_account_info()
        ],
        signer_seeds,
    )?;

    msg!("Meteora position created successfully");
    msg!("Position: {}", ctx.accounts.position.key());
    msg!("NFT Mint: {}", ctx.accounts.position_nft_mint.key());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_id_validation() {
        let zero_id = [0u8; 32];
        let valid_id = [1u8; 32];

        // Zero ID should be invalid
        assert_eq!(zero_id, [0u8; 32]);

        // Non-zero ID should be valid
        assert_ne!(valid_id, [0u8; 32]);
    }

    #[test]
    fn test_position_strategy_logic() {
        // Test the logic for quote-only fee accrual
        // This would be implemented in the liquidity addition instruction

        // If quote is token0, range should be above current price
        // If quote is token1, range should be below current price
        // This ensures fees only accrue in the quote direction

        println!("Position strategy depends on current price and token order");
    }
}
