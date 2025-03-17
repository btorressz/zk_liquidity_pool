use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::prelude::*;

declare_id!("9GAC41pniqSKXbGmQ5jzGrbZDgVTz3U7Mt2jmQ3hePyv");

// ---------------------------------------------------------------------
// Dummy types & helper modules for SPL Confidential Tokens and ZK proofs
// ---------------------------------------------------------------------

/// A marker type for the Confidential Token Program.
#[derive(Clone)]
pub struct ConfidentialTokenProgram;
impl anchor_lang::Id for ConfidentialTokenProgram {
    fn id() -> Pubkey {
        // Replace with the actual SPL Confidential Token Program ID.
        Pubkey::new_from_array([0u8; 32])
    }
}

/// Compression utilities for large ZK proofs.
fn compress_proof(proof: Vec<u8>) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&proof).map_err(|_| ErrorCode::CompressionError)?;
    let compressed_data = encoder.finish().map_err(|_| ErrorCode::CompressionError)?;
    Ok(compressed_data)
}

fn decompress_proof(compressed: Vec<u8>) -> Result<Vec<u8>> {
    // TODO: Implement decompression logic using flate2 if needed.
    Ok(compressed) // Placeholder: return input directly.
}

pub mod zk_utils {
    use super::*;

    /// Verifies a zero-knowledge proof for balance updates.
    pub fn verify_confidential_balance(
        zk_proof: Vec<u8>,
        amount: u64,
        old_balance: [u8; 64],
        new_balance: [u8; 64],
    ) -> Result<()> {
        // TODO: Integrate a zk-SNARK/zk-STARK verifier (e.g., halo2, gnark, or circom)
        // to process proofs off-chain. Ensure that old_balance + amount = new_balance without revealing values.
        if zk_proof.is_empty() {
            return Err(ErrorCode::InvalidZKProof.into());
        }
        Ok(())
    }

    /// Verifies a zero-knowledge proof for confidential transfers.
    pub fn verify_transfer_proof(zk_proof: Vec<u8>) -> Result<()> {
        // TODO: Implement range proofs using Bulletproofs or Groth16 to validate transfers without leaking amounts.
        if zk_proof.is_empty() {
            return Err(ErrorCode::InvalidZKProof.into());
        }
        Ok(())
    }

    /// Verifies a zero-knowledge identity proof to prevent Sybil attacks.
    pub fn verify_identity_proof(zk_identity_proof: Vec<u8>) -> Result<()> {
        // TODO: Store Merkle tree roots for user identity commitments and use zk-proofs (zk-SNARK/zk-STARK)
        // to ensure unique identity, preventing multi-account farming and fraud.
        if zk_identity_proof.is_empty() {
            return Err(ErrorCode::SybilAttackDetected.into());
        }
        Ok(())
    }
}

/// Dummy confidential transfer call using CPI to the confidential token program.
/// In production, replace this stub with a call to spl_confidential_token::instruction::confidential_transfer(),
/// ensuring that the from, to, and authority accounts are validated.
fn confidential_transfer(
    ct_program: &Program<ConfidentialTokenProgram>,
    from: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
) -> Result<()> {
    // TODO: Invoke spl_confidential_token::instruction::confidential_transfer() here,
    // ensuring the destination balance commitment is securely updated.
    Ok(())
}

/// Dummy confidential transfer call that accepts a PDA signer.
/// In production, modify this function to use invoke_signed() with proper PDA authorization.
fn confidential_transfer_with_signer(
    ct_program: &Program<ConfidentialTokenProgram>,
    from: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    // TODO: Use invoke_signed() with seeds-based PDAs to ensure only legitimate pool contracts can execute this transfer.
    Ok(())
}

/// Dummy confidential mint instruction for reward distribution.
/// In production, call spl_confidential_token::instruction::confidential_mint() to confidentially mint tokens.
fn confidential_mint(
    ct_program: &Program<ConfidentialTokenProgram>,
    to: &AccountInfo,
    amount: u64,
) -> Result<()> {
    // TODO: Call the confidential mint instruction from the SPL Confidential Token program.
    Ok(())
}

// ---------------------------------------------------------------------
// Program Declaration & Instruction Handlers
// ---------------------------------------------------------------------

#[program]
pub mod zk_liquidity_pool {
    use super::*;

    /// Initializes the liquidity pool.
    pub fn initialize_pool(ctx: Context<InitializePool>, bump: u8) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        pool.authority = *ctx.accounts.authority.key;
        pool.token_mint = ctx.accounts.token_mint.key();
        pool.total_staked = 0;
        pool.bump = bump;
        Ok(())
    }

    /// Stake tokens into the pool using confidential transfers with multi-asset support.
    /// Stores the asset type and stake timestamp for flash loan protection.
    pub fn stake(
        ctx: Context<StakeAccounts>,
        amount: u64,
        zk_proof: Vec<u8>,
        new_confidential_balance: [u8; 64],
    ) -> Result<()> {
        // Verify the provided ZK proof for the stake.
        zk_utils::verify_confidential_balance(
            zk_proof.clone(),
            amount,
            ctx.accounts.user_stake.confidential_balance,
            new_confidential_balance,
        )?;

        // Optionally compress the proof to save space.
        let _compressed_proof = compress_proof(zk_proof.clone()).ok();

        // Perform a confidential transfer from the user's account to the pool's account.
        confidential_transfer(
            &ctx.accounts.confidential_token_program,
            ctx.accounts.user_token_account.to_account_info().as_ref(),
            ctx.accounts.pool_token_account.to_account_info().as_ref(),
            ctx.accounts.user.to_account_info().as_ref(),
            amount,
        )?;

        // Update the user's confidential balance commitment.
        ctx.accounts.user_stake.confidential_balance = new_confidential_balance;
        // Record the current timestamp for flash loan resistance.
        ctx.accounts.user_stake.stake_timestamp = Clock::get()?.unix_timestamp;
        // Set the asset type for multi-asset staking.
        ctx.accounts.user_stake.asset_mint = ctx.accounts.token_mint.key();

        // Update the pool's total staked amount (kept in plaintext for reward calculation).
        let pool = &mut ctx.accounts.pool;
        pool.total_staked = pool
            .total_staked
            .checked_add(amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Withdraw staked tokens.
    /// Ensures that the stake has been held for a minimum duration to prevent flash loan attacks.
    pub fn withdraw(
        ctx: Context<Withdraw>,
        amount: u64,
        zk_proof: Vec<u8>,
        new_confidential_balance: [u8; 64],
    ) -> Result<()> {
        // Check that minimum staking duration has passed.
        let current_time = Clock::get()?.unix_timestamp;
        let stake_time = ctx.accounts.user_stake.stake_timestamp;
        let min_duration: i64 = 60; // e.g., a minimum 60-second lockup (adjust as needed)
        if current_time - stake_time < min_duration {
            return Err(ErrorCode::StakeDurationNotMet.into());
        }

        // Verify the ZK proof for the withdrawal.
        zk_utils::verify_confidential_balance(
            zk_proof,
            amount,
            ctx.accounts.user_stake.confidential_balance,
            new_confidential_balance,
        )?;

        // Derive PDA seeds for the pool authority.
        let pool = &ctx.accounts.pool;
        let pool_key = pool.key();
        let seeds = &[b"pool".as_ref(), pool_key.as_ref(), &[pool.bump]];
        let signer = &[&seeds[..]];

        // Perform the confidential transfer from the pool back to the user.
        confidential_transfer_with_signer(
            &ctx.accounts.confidential_token_program,
            ctx.accounts.pool_token_account.to_account_info().as_ref(),
            ctx.accounts.user_token_account.to_account_info().as_ref(),
            ctx.accounts.pool.to_account_info().as_ref(),
            amount,
            signer,
        )?;

        // Update the user's confidential balance commitment.
        ctx.accounts.user_stake.confidential_balance = new_confidential_balance;

        // Update the pool's total staked amount.
        ctx.accounts.pool.total_staked = ctx
            .accounts
            .pool
            .total_staked
            .checked_sub(amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Commit a swap order using a commit–reveal scheme.
    /// The trader commits to an order by providing a commitment hash and an encrypted order.
    pub fn commit_swap(
        ctx: Context<CommitSwap>,
        commitment: [u8; 32],
        encrypted_order: [u8; 64],
    ) -> Result<()> {
        let trade_order = &mut ctx.accounts.trade_order;
        trade_order.commitment = commitment;
        trade_order.encrypted_order = encrypted_order;
        trade_order.trade_timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }

    /// Reveal the trade order, verifying the commitment with a provided ZK proof.
    pub fn reveal_swap(
        ctx: Context<RevealSwap>,
        zk_proof: Vec<u8>,
        order_details: Vec<u8>, // Decrypted order details.
    ) -> Result<()> {
        // TODO: Implement a commitment scheme with a time delay (e.g., zk-time locks) to prevent premature reveal.
        // Verify that hash(order_details + secret nonce) matches the stored commitment.
        zk_utils::verify_transfer_proof(zk_proof)?;
        ctx.accounts.trade_order.revealed_order = order_details;
        Ok(())
    }

    /// Distribute rewards to liquidity providers using confidential minting.
    /// Incorporate zk-SNARK-based reward calculation to preserve privacy.
    pub fn distribute_rewards(
        ctx: Context<DistributeRewards>,
        zk_reward_proof: Vec<u8>,
        reward_amount: u64,
    ) -> Result<()> {
        // Verify the ZK proof for reward distribution.
        zk_utils::verify_transfer_proof(zk_reward_proof)?;
        confidential_mint(
            &ctx.accounts.confidential_token_program,
            ctx.accounts.pool_token_account.to_account_info().as_ref(),
            reward_amount,
        )?;
        Ok(())
    }

    /// Update reward parameters using multi-signature governance.
    /// Implements zk-SNARK voting where votes remain encrypted and only aggregated totals are revealed.
    pub fn update_reward_params(
        ctx: Context<UpdateRewardParams>,
        new_reward_rate: u64,
        zk_governance_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Verify multi-signature governance via a zk-enabled tallying system.
        zk_utils::verify_transfer_proof(zk_governance_proof)?;
        ctx.accounts.governance.reward_rate = new_reward_rate;
        Ok(())
    }

    /// Cast a confidential vote on governance issues.
    pub fn confidential_vote(
        ctx: Context<ConfidentialVote>,
        vote: u8,
        zk_vote_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Implement confidential voting logic using zk-SNARK proofs so that individual votes remain hidden.
        zk_utils::verify_identity_proof(zk_vote_proof)?;
        ctx.accounts.governance.vote_count = ctx
            .accounts
            .governance
            .vote_count
            .checked_add(vote as u64)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Additional Advanced Features
    // ---------------------------------------------------------------------

    /// zk-enabled multi-signature transaction.
    pub fn zk_multisig_transaction(
        ctx: Context<AdditionalFeatures>,
        multisig_data: Vec<u8>,
        zk_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Implement multi-signature approvals using zk-SNARKs so that multiple parties can sign without revealing their identities.
        zk_utils::verify_transfer_proof(zk_proof)?;
        Ok(())
    }

    /// ZK rollback protection to prevent transaction replay or reversion.
    pub fn zk_rollback_protection(
        ctx: Context<AdditionalFeatures>,
        zk_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Implement zk-proofs to ensure the transaction is not being replayed or fraudulently reverted.
        zk_utils::verify_transfer_proof(zk_proof)?;
        Ok(())
    }

    /// Batch staking multiple assets in a single confidential transaction using zk-proofs.
    pub fn batch_stake(
        ctx: Context<AdditionalFeatures>,
        amounts: Vec<u64>,
        zk_proofs: Vec<Vec<u8>>,
        new_confidential_balances: Vec<[u8; 64]>,
    ) -> Result<()> {
        // TODO: Loop through each stake, validate each using zk-proofs, and execute confidential transfers.
        Ok(())
    }

    /// zk-enabled exit mechanism for LPs to withdraw liquidity without revealing exact shares.
    pub fn zk_exit(
        ctx: Context<AdditionalFeatures>,
        amount: u64,
        zk_proof: Vec<u8>,
        new_confidential_balance: [u8; 64],
    ) -> Result<()> {
        // TODO: Implement a zk-enabled exit that verifies withdrawal without exposing the precise stake.
        zk_utils::verify_confidential_balance(
            zk_proof,
            amount,
            ctx.accounts.user_stake.confidential_balance,
            new_confidential_balance,
        )?;
        Ok(())
    }

    /// zk-based automatic liquidity rebalancing.
    pub fn zk_auto_rebalance(
        ctx: Context<AdditionalFeatures>,
        zk_proof: Vec<u8>,
        liquidity_params: Vec<u8>,
    ) -> Result<()> {
        // TODO: Implement automatic liquidity rebalancing using zk-proofs to adjust pool parameters confidentially.
        zk_utils::verify_transfer_proof(zk_proof)?;
        Ok(())
    }

    /// zk-time lock unlocking mechanism for liquidity.
    pub fn zk_time_lock_unlock(
        ctx: Context<AdditionalFeatures>,
        zk_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Use zk-time locks to allow liquidity unlocking only after a specified delay.
        zk_utils::verify_transfer_proof(zk_proof)?;
        Ok(())
    }

    /// Place a confidential limit order using zk-proofs.
    pub fn confidential_limit_order(
        ctx: Context<AdditionalFeatures>,
        order_data: Vec<u8>,
        zk_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Implement confidential limit orders, allowing users to set trade conditions privately.
        zk_utils::verify_transfer_proof(zk_proof)?;
        Ok(())
    }

    /// zk-secured smart contract upgradability.
    pub fn zk_upgrade(
        ctx: Context<AdditionalFeatures>,
        upgrade_data: Vec<u8>,
        zk_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Implement contract upgrade validation using zk-proofs for governance-approved changes.
        zk_utils::verify_transfer_proof(zk_proof)?;
        Ok(())
    }

    /// zk-proof of funds verification to confirm user holds required funds confidentially.
    pub fn zk_proof_of_funds(
        ctx: Context<AdditionalFeatures>,
        zk_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Verify via zk-proofs that the user holds the required funds without exposing the actual balance.
        zk_utils::verify_transfer_proof(zk_proof)?;
        Ok(())
    }

    /// zk-private flash loans where loan details remain confidential until repayment.
    pub fn zk_private_flash_loan(
        ctx: Context<AdditionalFeatures>,
        loan_amount: u64,
        zk_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Implement private flash loans using zk-proofs to conceal loan amounts and terms until settlement.
        zk_utils::verify_transfer_proof(zk_proof)?;
        Ok(())
    }

    /// Integrate zk-Rollups to batch confidential transactions and reduce transaction fees.
    pub fn integrate_zk_rollup(ctx: Context<AdditionalFeatures>) -> Result<()> {
        // TODO: Implement zk-Rollup integration to improve scalability and reduce transaction fees.
        Ok(())
    }

    /// Implement private order matching in the AMM using zk-proofs.
    pub fn private_order_matching(ctx: Context<AdditionalFeatures>) -> Result<()> {
        // TODO: Create a private order book via zk-proofs so that liquidity levels remain confidential.
        Ok(())
    }

    /// Enable private lending and borrowing markets using zk-enabled credit scores.
    pub fn private_lending(ctx: Context<AdditionalFeatures>) -> Result<()> {
        // TODO: Implement confidential lending pools without revealing borrower identities.
        Ok(())
    }

    /// Introduce zk-proof staking challenges for LPs.
    pub fn zk_proof_staking_challenges(ctx: Context<AdditionalFeatures>) -> Result<()> {
        // TODO: Allow LPs to contest suspicious liquidity changes using zk-proofs without revealing full stake details.
        Ok(())
    }

    /// Display on-chain liquidity privacy metrics by showing aggregate liquidity without exposing individual positions.
    pub fn onchain_liquidity_privacy_metrics(ctx: Context<AdditionalFeatures>) -> Result<()> {
        // TODO: Implement on-chain metrics that protect individual LP privacy.
        Ok(())
    }

    /// Enable zk-encrypted messaging for LP coordination.
    pub fn zk_encrypted_messaging(
        ctx: Context<AdditionalFeatures>,
        message: Vec<u8>,
        zk_proof: Vec<u8>,
    ) -> Result<()> {
        // TODO: Develop an off-chain zk-enabled messaging system for LP coordination.
        zk_utils::verify_identity_proof(zk_proof)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Account Definitions
// ---------------------------------------------------------------------

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + LiquidityPool::LEN,
        seeds = [b"pool", token_mint.key().as_ref()],
        bump,
    )]
    pub pool: Account<'info, LiquidityPool>,
    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// The liquidity pool configuration.
#[account]
pub struct LiquidityPool {
    pub authority: Pubkey,
    pub token_mint: Pubkey,
    pub total_staked: u64,
    pub bump: u8,
    // Additional configuration fields (e.g., fee rate, trade volume) can be added here.
}

impl LiquidityPool {
    // Total space: 32 (authority) + 32 (mint) + 8 (u64) + 1 (bump) = 73 bytes.
    pub const LEN: usize = 32 + 32 + 8 + 1;
}

#[derive(Accounts)]
pub struct StakeAccounts<'info> {
    #[account(mut, has_one = token_mint)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + UserStake::LEN,
        seeds = [b"user_stake", user.key().as_ref(), pool.key().as_ref()],
        bump,
    )]
    pub user_stake: Account<'info, UserStake>,
    #[account(mut)]
    pub user_token_account: Account<'info, ConfidentialTokenAccount>,
    #[account(mut)]
    pub pool_token_account: Account<'info, ConfidentialTokenAccount>,
    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub confidential_token_program: Program<'info, ConfidentialTokenProgram>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// The user stake account stores a confidential balance commitment, stake timestamp, and asset type.
#[account]
pub struct UserStake {
    pub confidential_balance: [u8; 64], // Zero-knowledge balance commitment
    pub encrypted_data: Vec<u8>,          // Optional encrypted metadata
    pub stake_timestamp: i64,             // Timestamp for flash loan protection
    pub asset_mint: Pubkey,               // The mint of the staked asset (for multi-asset support)
}

impl UserStake {
    // Total space: 64 + 4 + 64 + 8 + 32 = 172 bytes.
    pub const LEN: usize = 64 + 4 + 64 + 8 + 32;
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, has_one = token_mint)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut, seeds = [b"user_stake", user.key().as_ref(), pool.key().as_ref()], bump)]
    pub user_stake: Account<'info, UserStake>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool_token_account: Account<'info, ConfidentialTokenAccount>,
    #[account(mut)]
    pub user_token_account: Account<'info, ConfidentialTokenAccount>,
    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub confidential_token_program: Program<'info, ConfidentialTokenProgram>,
}

#[derive(Accounts)]
pub struct CommitSwap<'info> {
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(
        init,
        payer = trader,
        space = 8 + TradeOrder::LEN,
        seeds = [b"trade_order", trader.key().as_ref(), pool.key().as_ref()],
        bump,
    )]
    pub trade_order: Account<'info, TradeOrder>,
    #[account(mut)]
    pub trader: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct RevealSwap<'info> {
    #[account(mut, seeds = [b"trade_order", trader.key().as_ref(), pool.key().as_ref()], bump)]
    pub trade_order: Account<'info, TradeOrder>,
    #[account(mut)]
    pub trader: Signer<'info>,
    pub pool: Account<'info, LiquidityPool>,
}

#[derive(Accounts)]
pub struct DistributeRewards<'info> {
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub pool_token_account: Account<'info, ConfidentialTokenAccount>,
    pub token_program: Program<'info, Token>,
    pub confidential_token_program: Program<'info, ConfidentialTokenProgram>,
}

#[derive(Accounts)]
pub struct UpdateRewardParams<'info> {
    #[account(mut, has_one = authority)]
    pub governance: Account<'info, Governance>,
    #[account(mut)]
    pub authority: Signer<'info>,
    // Additional signers can be added here for multi-signature verification.
}

#[derive(Accounts)]
pub struct ConfidentialVote<'info> {
    #[account(mut)]
    pub governance: Account<'info, Governance>,
    #[account(mut)]
    pub voter: Signer<'info>,
}

/// Governance account for multi-signature reward parameter updates and confidential voting.
#[account]
pub struct Governance {
    pub authority: Pubkey,
    pub reward_rate: u64,
    pub vote_count: u64,
    // Additional governance parameters can be added here.
}

/// A placeholder for confidential token accounts.
/// In production, these would be managed by the SPL Confidential Token program.
#[account]
pub struct ConfidentialTokenAccount {
    pub balance_commitment: [u8; 64],
    // Additional fields for confidential tokens can be added here.
}

/// Account representing a trade order commitment for commit–reveal swap execution.
#[account]
pub struct TradeOrder {
    pub commitment: [u8; 32],
    pub encrypted_order: [u8; 64],
    pub trade_timestamp: i64,
    pub revealed_order: Vec<u8>, // Optional: store revealed order details after commit–reveal.
}

impl TradeOrder {
    // Total space: 32 + 64 + 8 + 4 + 128 = 236 bytes.
    pub const LEN: usize = 32 + 64 + 8 + 4 + 128;
}

/// Placeholder for additional zk-account action context.
#[derive(Accounts)]
pub struct SomeZkAccountAction<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    // Additional accounts as needed.
}

/// Accounts context for additional advanced features.
#[derive(Accounts)]
pub struct AdditionalFeatures<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)] // Added: reference to the user's stake account
    pub user_stake: Account<'info, UserStake>,
    // Additional accounts as needed.
}

// ---------------------------------------------------------------------
// Error Codes
// ---------------------------------------------------------------------

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient stake to withdraw the requested amount.")]
    InsufficientStake,
    #[msg("Mathematical overflow occurred.")]
    MathOverflow,
    #[msg("Invalid zero-knowledge proof provided.")]
    InvalidZKProof,
    #[msg("Sybil attack detected.")]
    SybilAttackDetected,
    #[msg("Stake duration not met for withdrawal.")]
    StakeDurationNotMet,
    #[msg("Compression error occurred.")]
    CompressionError,
}
