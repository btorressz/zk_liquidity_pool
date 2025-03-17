//Exported from Solana Playground
//This test file needs to be edited 

import * as anchor from "@coral-xyz/anchor";
import { Program, web3, BN } from "@coral-xyz/anchor";
import assert from "assert";
import type { ZkLiquidityPool } from "../target/types/zk_liquidity_pool";

describe("zk-liquidity-pool", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.ZkLiquidityPool as Program<ZkLiquidityPool>;
  const provider = program.provider;

  it("initializes the liquidity pool", async () => {
    // Generate a dummy token mint
    const tokenMint = web3.Keypair.generate();

    // Derive the pool PDA using seed ["pool", tokenMint]
    const [poolPDA, bump] = await web3.PublicKey.findProgramAddress(
      [Buffer.from("pool"), tokenMint.publicKey.toBuffer()],
      program.programId
    );

    // Call initialize_pool with bump and proper accounts.
    const tx = await program.methods.initializePool(new BN(bump))
      .accounts({
        pool: poolPDA,
        tokenMint: tokenMint.publicKey,
        authority: provider.publicKey,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Transaction signature:", tx);

    // Fetch the pool account and verify initialization
    const poolAccount = await program.account.liquidityPool.fetch(poolPDA);
    assert.ok(poolAccount.totalStaked.eq(new BN(0)));
    assert.ok(poolAccount.bump === bump);
    assert.ok(poolAccount.tokenMint.equals(tokenMint.publicKey));
    assert.ok(poolAccount.authority.equals(provider.publicKey));
  });
});
