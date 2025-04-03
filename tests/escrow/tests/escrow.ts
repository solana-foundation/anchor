import * as anchor from "@coral-xyz/anchor";
import { Program, BN, IdlAccounts } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { assert } from "chai";
import { Escrow } from "../target/types/escrow";

type EscrowAccount = IdlAccounts<Escrow>["escrowAccount"];

describe("escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const TOKEN_2022_PROGRAM_ID = new anchor.web3.PublicKey(
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
  );

  const TEST_PROGRAM_IDS = [
    [TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID],
    [TOKEN_2022_PROGRAM_ID, TOKEN_2022_PROGRAM_ID],
    [TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID],
  ];

  const program = anchor.workspace.Escrow as Program<Escrow>;

  const payer = Keypair.generate();
  const mintAuthority = Keypair.generate();

  const takerAmount = 1000;
  const initializerAmount = 500;

  let mintA: Token;
  let mintB: Token;
  let initializerTokenAccountA: PublicKey;
  let initializerTokenAccountB: PublicKey;
  let takerTokenAccountA: PublicKey;
  let takerTokenAccountB: PublicKey;
  let pda: PublicKey;

  const airdropSol = async (publicKey: PublicKey, amount: number) => {
    const signature = await provider.connection.requestAirdrop(publicKey, amount);
    await provider.connection.confirmTransaction(signature, "confirmed");
  };

  const createMint = async (tokenProgramId: PublicKey) => {
    return await Token.createMint(
      provider.connection,
      payer,
      mintAuthority.publicKey,
      null,
      0,
      tokenProgramId
    );
  };

  const createTokenAccount = async (mint: Token, owner: PublicKey) => {
    return await mint.createAccount(owner);
  };

  const mintToAccount = async (mint: Token, account: PublicKey, amount: number) => {
    await mint.mintTo(account, mintAuthority.publicKey, [mintAuthority], amount);
  };

  TEST_PROGRAM_IDS.forEach(([tokenProgramIdA, tokenProgramIdB]) => {
    const escrowAccount = Keypair.generate();
    const testName =
      tokenProgramIdA === tokenProgramIdB
        ? tokenProgramIdA.equals(TOKEN_PROGRAM_ID)
          ? "token"
          : "token-2022"
        : "mixed";

    describe(testName, () => {
      before(async () => {
        await airdropSol(payer.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);

        mintA = await createMint(tokenProgramIdA);
        mintB = await createMint(tokenProgramIdB);

        initializerTokenAccountA = await createTokenAccount(mintA, provider.wallet.publicKey);
        takerTokenAccountA = await createTokenAccount(mintA, provider.wallet.publicKey);
        initializerTokenAccountB = await createTokenAccount(mintB, provider.wallet.publicKey);
        takerTokenAccountB = await createTokenAccount(mintB, provider.wallet.publicKey);

        await mintToAccount(mintA, initializerTokenAccountA, initializerAmount);
        await mintToAccount(mintB, takerTokenAccountB, takerAmount);

        const initializerTokenAccountAInfo = await mintA.getAccountInfo(initializerTokenAccountA);
        const takerTokenAccountBInfo = await mintB.getAccountInfo(takerTokenAccountB);

        assert.strictEqual(initializerTokenAccountAInfo.amount.toNumber(), initializerAmount);
        assert.strictEqual(takerTokenAccountBInfo.amount.toNumber(), takerAmount);
      });

      it("Initializes escrow", async () => {
        await program.rpc.initializeEscrow(new BN(initializerAmount), new BN(takerAmount), {
          accounts: {
            initializer: provider.wallet.publicKey,
            initializerDepositTokenAccount: initializerTokenAccountA,
            initializerReceiveTokenAccount: initializerTokenAccountB,
            escrowAccount: escrowAccount.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: tokenProgramIdA,
          },
          signers: [escrowAccount],
        });

        const [escrowPda] = await PublicKey.findProgramAddress(
          [Buffer.from(anchor.utils.bytes.utf8.encode("escrow"))],
          program.programId
        );
        pda = escrowPda;

        const initializerTokenAccountAInfo = await mintA.getAccountInfo(initializerTokenAccountA);
        const escrowAccountInfo = await program.account.escrowAccount.fetch(escrowAccount.publicKey);

        assert.isTrue(initializerTokenAccountAInfo.owner.equals(pda));
        assert.isTrue(escrowAccountInfo.initializerKey.equals(provider.wallet.publicKey));
        assert.strictEqual(escrowAccountInfo.initializerAmount.toNumber(), initializerAmount);
        assert.strictEqual(escrowAccountInfo.takerAmount.toNumber(), takerAmount);
      });

      it("Exchanges escrow", async () => {
        await program.rpc.exchange({
          accounts: {
            taker: provider.wallet.publicKey,
            takerDepositTokenAccount: takerTokenAccountB,
            takerReceiveTokenAccount: takerTokenAccountA,
            pdaDepositTokenAccount: initializerTokenAccountA,
            initializerReceiveTokenAccount: initializerTokenAccountB,
            initializerMainAccount: provider.wallet.publicKey,
            escrowAccount: escrowAccount.publicKey,
            pdaAccount: pda,
            depositMint: mintB.publicKey,
            receiveMint: mintA.publicKey,
            depositTokenProgram: tokenProgramIdB,
            receiveTokenProgram: tokenProgramIdA,
          },
        });

        const takerTokenAccountAInfo = await mintA.getAccountInfo(takerTokenAccountA);
        const takerTokenAccountBInfo = await mintB.getAccountInfo(takerTokenAccountB);
        const initializerTokenAccountAInfo = await mintA.getAccountInfo(initializerTokenAccountA);
        const initializerTokenAccountBInfo = await mintB.getAccountInfo(initializerTokenAccountB);

        assert.strictEqual(takerTokenAccountAInfo.amount.toNumber(), initializerAmount);
        assert.strictEqual(takerTokenAccountBInfo.amount.toNumber(), 0);
        assert.strictEqual(initializerTokenAccountAInfo.amount.toNumber(), 0);
        assert.strictEqual(initializerTokenAccountBInfo.amount.toNumber(), takerAmount);
      });

      it("Cancels escrow", async () => {
        await program.rpc.cancelEscrow({
          accounts: {
            initializer: provider.wallet.publicKey,
            pdaDepositTokenAccount: initializerTokenAccountA,
            pdaAccount: pda,
            escrowAccount: escrowAccount.publicKey,
            tokenProgram: tokenProgramIdA,
          },
        });

        const initializerTokenAccountAInfo = await mintA.getAccountInfo(initializerTokenAccountA);
        assert.isTrue(initializerTokenAccountAInfo.owner.equals(provider.wallet.publicKey));
        assert.strictEqual(initializerTokenAccountAInfo.amount.toNumber(), initializerAmount);
      });
    });
  });
});
