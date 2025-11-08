import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { VaultManager } from "../target/types/vault_manager";
import { 
  PublicKey, 
  Keypair, 
  SystemProgram,
  LAMPORTS_PER_SOL 
} from "@solana/web3.js";
import { 
  TOKEN_PROGRAM_ID, 
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { expect } from "chai";

describe("vault-manager", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.VaultManager as Program<VaultManager>;
  
  let usdtMint: PublicKey;
  let user: Keypair;
  let userTokenAccount: PublicKey;
  let vaultPda: PublicKey;
  let vaultBump: number;
  let vaultTokenAccount: PublicKey;
  let authorityPda: PublicKey;
  let authorityBump: number;

  before(async () => {
    // Create test user
    user = Keypair.generate();
    
    // Airdrop SOL to user
    const signature = await provider.connection.requestAirdrop(
      user.publicKey,
      2 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(signature);

    // Create USDT mint
    usdtMint = await createMint(
      provider.connection,
      user,
      user.publicKey,
      null,
      6 // 6 decimals like USDT
    );

    console.log("USDT Mint:", usdtMint.toBase58());

    // Create user token account
    const userTokenAccountInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      user,
      usdtMint,
      user.publicKey
    );
    userTokenAccount = userTokenAccountInfo.address;

    // Mint some USDT to user
    await mintTo(
      provider.connection,
      user,
      usdtMint,
      userTokenAccount,
      user.publicKey,
      10000 * 1e6 // 10,000 USDT
    );

    console.log("User Token Account:", userTokenAccount.toBase58());

    // Derive PDAs
    [vaultPda, vaultBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), user.publicKey.toBuffer()],
      program.programId
    );

    [authorityPda, authorityBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("authority")],
      program.programId
    );

    console.log("Vault PDA:", vaultPda.toBase58());
    console.log("Authority PDA:", authorityPda.toBase58());
  });

  it("Initializes vault authority", async () => {
    const tx = await program.methods
      .initializeAuthority()
      .accounts({
        admin: provider.wallet.publicKey,
        authority: authorityPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Initialize authority tx:", tx);

    const authorityAccount = await program.account.vaultAuthority.fetch(authorityPda);
    expect(authorityAccount.admin.toBase58()).to.equal(
      provider.wallet.publicKey.toBase58()
    );
  });

  it("Initializes user vault", async () => {
    const tx = await program.methods
      .initializeVault()
      .accounts({
        user: user.publicKey,
        vault: vaultPda,
        vaultTokenAccount: vaultTokenAccount,
        mint: usdtMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    console.log("Initialize vault tx:", tx);

    const vaultAccount = await program.account.collateralVault.fetch(vaultPda);
    expect(vaultAccount.owner.toBase58()).to.equal(user.publicKey.toBase58());
    expect(vaultAccount.totalBalance.toNumber()).to.equal(0);
  });

  it("Deposits collateral", async () => {
    const depositAmount = new anchor.BN(1000 * 1e6); // 1,000 USDT

    const tx = await program.methods
      .deposit(depositAmount)
      .accounts({
        user: user.publicKey,
        vault: vaultPda,
        userTokenAccount: userTokenAccount,
        vaultTokenAccount: vaultTokenAccount,
        owner: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    console.log("Deposit tx:", tx);

    const vaultAccount = await program.account.collateralVault.fetch(vaultPda);
    expect(vaultAccount.totalBalance.toNumber()).to.equal(depositAmount.toNumber());
    expect(vaultAccount.availableBalance.toNumber()).to.equal(depositAmount.toNumber());
  });

  it("Locks collateral", async () => {
    const lockAmount = new anchor.BN(500 * 1e6); // 500 USDT

    const tx = await program.methods
      .lockCollateral(lockAmount)
      .accounts({
        vault: vaultPda,
        authority: authorityPda,
      })
      .rpc();

    console.log("Lock collateral tx:", tx);

    const vaultAccount = await program.account.collateralVault.fetch(vaultPda);
    expect(vaultAccount.lockedBalance.toNumber()).to.equal(lockAmount.toNumber());
    expect(vaultAccount.availableBalance.toNumber()).to.equal(500 * 1e6);
  });

  it("Unlocks collateral", async () => {
    const unlockAmount = new anchor.BN(200 * 1e6); // 200 USDT

    const tx = await program.methods
      .unlockCollateral(unlockAmount)
      .accounts({
        vault: vaultPda,
        authority: authorityPda,
      })
      .rpc();

    console.log("Unlock collateral tx:", tx);

    const vaultAccount = await program.account.collateralVault.fetch(vaultPda);
    expect(vaultAccount.lockedBalance.toNumber()).to.equal(300 * 1e6);
    expect(vaultAccount.availableBalance.toNumber()).to.equal(700 * 1e6);
  });

  it("Withdraws collateral", async () => {
    const withdrawAmount = new anchor.BN(500 * 1e6); // 500 USDT

    const tx = await program.methods
      .withdraw(withdrawAmount)
      .accounts({
        user: user.publicKey,
        vault: vaultPda,
        userTokenAccount: userTokenAccount,
        vaultTokenAccount: vaultTokenAccount,
        owner: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    console.log("Withdraw tx:", tx);

    const vaultAccount = await program.account.collateralVault.fetch(vaultPda);
    expect(vaultAccount.totalBalance.toNumber()).to.equal(500 * 1e6);
    expect(vaultAccount.totalWithdrawn.toNumber()).to.equal(withdrawAmount.toNumber());
  });

  it("Fails to withdraw with insufficient balance", async () => {
    const withdrawAmount = new anchor.BN(10000 * 1e6); // More than available

    try {
      await program.methods
        .withdraw(withdrawAmount)
        .accounts({
          user: user.publicKey,
          vault: vaultPda,
          userTokenAccount: userTokenAccount,
          vaultTokenAccount: vaultTokenAccount,
          owner: user.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();
      
      expect.fail("Should have thrown an error");
    } catch (error) {
      expect(error).to.exist;
    }
  });
});
