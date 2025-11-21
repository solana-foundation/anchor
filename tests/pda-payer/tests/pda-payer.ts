import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PdaPayer } from "../target/types/pda_payer";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import { expect } from "chai";

describe("pda-payer", () => {
  // Configure the client to use the local cluster.
  const connection = new anchor.web3.Connection("http://127.0.0.1:8899", "confirmed");
  const wallet = new anchor.Wallet(Keypair.generate());
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);
  
  // Airdrop some SOL to the wallet for testing
  before(async () => {
    const airdropSig = await connection.requestAirdrop(
      wallet.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(airdropSig);
  });

  const program = anchor.workspace.PdaPayer as Program<PdaPayer>;

  it("Initializes account with PDA as payer", async () => {
    // Find PDA address
    const [pdaAccount, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from("my-pda")],
      program.programId
    );

    // Fund the PDA account so it can pay for the new account
    // We need to transfer funds from the wallet to the PDA since PDAs can't receive airdrops
    const fundTx = await provider.sendAndConfirm(
      new anchor.web3.Transaction().add(
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: pdaAccount,
          lamports: 2 * anchor.web3.LAMPORTS_PER_SOL,
        })
      )
    );

    // Derive the new account address (it will be created by the init constraint)
    // For init, we need to provide a keypair for the new account
    const newAccount = Keypair.generate();
    
    // Get the PDA account info to check it has funds
    const pdaAccountInfo = await provider.connection.getAccountInfo(pdaAccount);
    expect(pdaAccountInfo).to.not.be.null;
    expect(pdaAccountInfo!.lamports).to.be.greaterThan(0);

    try {
      // Call the instruction
      // The newAccount keypair is needed for init, but the PDA will pay for the account creation
      const tx = await program.methods
        .initWithPdaPayer()
        .accounts({
          pdaAccount: pdaAccount,
          newAccount: newAccount.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([newAccount])
        .rpc();

      console.log("Transaction signature:", tx);

      // Verify the new account was created
      const newAccountInfo = await provider.connection.getAccountInfo(
        newAccount.publicKey
      );
      expect(newAccountInfo).to.not.be.null;

      // Verify the account data
      const accountData = await program.account.myData.fetch(
        newAccount.publicKey
      );
      expect(accountData.data.toNumber()).to.equal(42);

      // Verify the PDA account was used as payer (its balance should have decreased)
      const pdaAccountInfoAfter = await provider.connection.getAccountInfo(
        pdaAccount
      );
      expect(pdaAccountInfoAfter!.lamports).to.be.lessThan(
        pdaAccountInfo!.lamports
      );
    } catch (err) {
      console.error("Error:", err);
      throw err;
    }
  });
});

