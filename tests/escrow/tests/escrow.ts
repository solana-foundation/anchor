import * as anchor from "@coral-xyz/anchor";
import { Program, BN, IdlAccounts } from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  createMint,
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { assert } from "chai";
import { Escrow } from "../target/types/escrow";

type EscrowAccount = IdlAccounts<Escrow>["escrowAccount"];

describe("escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const connection = provider.connection;

  const TEST_PROGRAM_IDS = [
    [TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID],
    [TOKEN_2022_PROGRAM_ID, TOKEN_2022_PROGRAM_ID],
    [TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID],
  ];
  const program = anchor.workspace.Escrow as Program<Escrow>;

  let mintA: PublicKey = null;
  let mintB: PublicKey = null;
  let initializerTokenAccountA: PublicKey = null;
  let initializerTokenAccountB: PublicKey = null;
  let takerTokenAccountA: PublicKey = null;
  let takerTokenAccountB: PublicKey = null;
  let pda: PublicKey = null;

  const takerAmount = 1000;
  const initializerAmount = 500;

  const payer = Keypair.generate();
  const mintAuthority = Keypair.generate();

  TEST_PROGRAM_IDS.forEach((tokenProgramIds) => {
    const escrowAccount = Keypair.generate();
    const [tokenProgramIdA, tokenProgramIdB] = tokenProgramIds;
    let name;
    if (tokenProgramIdA === tokenProgramIdB) {
      name = tokenProgramIdA === TOKEN_PROGRAM_ID ? "token" : "token-2022";
    } else {
      name = "mixed";
    }
    describe(name, () => {
      it("Initialise escrow state", async () => {
        // Airdropping tokens to a payer.
        await provider.connection.confirmTransaction(
          await provider.connection.requestAirdrop(
            payer.publicKey,
            10 * LAMPORTS_PER_SOL
          ),
          "confirmed"
        );

        [mintA, mintB] = await Promise.all(
          [
            { tokenProgram: tokenProgramIdA },
            { tokenProgram: tokenProgramIdB },
          ].map(({ tokenProgram }) =>
            createMint(
              connection,
              payer,
              mintAuthority.publicKey,
              undefined,
              0,
              undefined,
              undefined,
              tokenProgram
            )
          )
        );

        [
          initializerTokenAccountA,
          takerTokenAccountA,
          initializerTokenAccountB,
          takerTokenAccountB,
        ] = await Promise.all(
          [
            { mint: mintA, tokenProgram: tokenProgramIdA },
            { mint: mintA, tokenProgram: tokenProgramIdA },
            { mint: mintB, tokenProgram: tokenProgramIdB },
            { mint: mintB, tokenProgram: tokenProgramIdB },
          ].map(({ mint, tokenProgram }) =>
            createAccount(
              connection,
              payer,
              mint,
              provider.wallet.publicKey,
              Keypair.generate(),
              undefined,
              tokenProgram
            )
          )
        );

        await Promise.all([
          mintTo(
            connection,
            payer,
            mintA,
            initializerTokenAccountA,
            mintAuthority.publicKey,
            initializerAmount,
            [mintAuthority],
            undefined,
            tokenProgramIdA
          ),

          mintTo(
            connection,
            payer,
            mintB,
            takerTokenAccountB,
            mintAuthority.publicKey,
            takerAmount,
            [mintAuthority],
            undefined,
            tokenProgramIdB
          ),
        ]);

        let [_initializerTokenAccountA, _takerTokenAccountB] =
          await Promise.all([
            getAccount(
              connection,
              initializerTokenAccountA,
              undefined,
              tokenProgramIdA
            ),
            getAccount(
              connection,
              takerTokenAccountB,
              undefined,
              tokenProgramIdB
            ),
          ]);

        assert.strictEqual(
          _initializerTokenAccountA.amount,
          BigInt(initializerAmount)
        );
        assert.strictEqual(_takerTokenAccountB.amount, BigInt(takerAmount));
      });

      it("Initialize escrow", async () => {
        await program.rpc.initializeEscrow(
          new BN(initializerAmount),
          new BN(takerAmount),
          {
            accounts: {
              initializer: provider.wallet.publicKey,
              initializerDepositTokenAccount: initializerTokenAccountA,
              initializerReceiveTokenAccount: initializerTokenAccountB,
              escrowAccount: escrowAccount.publicKey,
              systemProgram: SystemProgram.programId,
              tokenProgram: tokenProgramIdA,
            },
            signers: [escrowAccount],
          }
        );

        // Get the PDA that is assigned authority to token account.
        const [_pda, _nonce] = PublicKey.findProgramAddressSync(
          [Buffer.from(anchor.utils.bytes.utf8.encode("escrow"))],
          program.programId
        );

        pda = _pda;

        let _initializerTokenAccountA = await getAccount(
          connection,
          initializerTokenAccountA,
          undefined,
          tokenProgramIdA
        );

        let _escrowAccount: EscrowAccount =
          await program.account.escrowAccount.fetch(escrowAccount.publicKey);

        // Check that the new owner is the PDA.
        assert.isTrue(_initializerTokenAccountA.owner.equals(pda));

        // Check that the values in the escrow account match what we expect.
        assert.isTrue(
          _escrowAccount.initializerKey.equals(provider.wallet.publicKey)
        );
        assert.strictEqual(
          _escrowAccount.initializerAmount.toNumber(),
          initializerAmount
        );
        assert.strictEqual(_escrowAccount.takerAmount.toNumber(), takerAmount);
        assert.isTrue(
          _escrowAccount.initializerDepositTokenAccount.equals(
            initializerTokenAccountA
          )
        );
        assert.isTrue(
          _escrowAccount.initializerReceiveTokenAccount.equals(
            initializerTokenAccountB
          )
        );
      });

      it("Exchange escrow", async () => {
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
            depositMint: mintB,
            receiveMint: mintA,
            depositTokenProgram: tokenProgramIdB,
            receiveTokenProgram: tokenProgramIdA,
          },
        });

        const accounts = [
          { address: takerTokenAccountA, tokenProgramId: tokenProgramIdA },
          { address: takerTokenAccountB, tokenProgramId: tokenProgramIdB },
          {
            address: initializerTokenAccountA,
            tokenProgramId: tokenProgramIdA,
          },
          {
            address: initializerTokenAccountB,
            tokenProgramId: tokenProgramIdB,
          },
        ];

        const [
          _takerTokenAccountA,
          _takerTokenAccountB,
          _initializerTokenAccountA,
          _initializerTokenAccountB,
        ] = await Promise.all(
          accounts.map(({ address, tokenProgramId }) =>
            getAccount(connection, address, undefined, tokenProgramId)
          )
        );

        // Check that the initializer gets back ownership of their token account.
        assert.isTrue(
          _takerTokenAccountA.owner.equals(provider.wallet.publicKey)
        );

        assert.strictEqual(
          _takerTokenAccountA.amount,
          BigInt(initializerAmount)
        );
        assert.strictEqual(_initializerTokenAccountA.amount, BigInt(0));
        assert.strictEqual(
          _initializerTokenAccountB.amount,
          BigInt(takerAmount)
        );
        assert.strictEqual(_takerTokenAccountB.amount, BigInt(0));
      });

      let newEscrow = Keypair.generate();

      it("Initialize escrow and cancel escrow", async () => {
        // Put back tokens into initializer token A account.
        await mintTo(
          connection,
          payer,
          mintA,
          initializerTokenAccountA,
          mintAuthority.publicKey,
          initializerAmount,
          [mintAuthority],
          undefined,
          tokenProgramIdA
        );

        await program.rpc.initializeEscrow(
          new BN(initializerAmount),
          new BN(takerAmount),
          {
            accounts: {
              initializer: provider.wallet.publicKey,
              initializerDepositTokenAccount: initializerTokenAccountA,
              initializerReceiveTokenAccount: initializerTokenAccountB,
              escrowAccount: newEscrow.publicKey,
              systemProgram: SystemProgram.programId,
              tokenProgram: tokenProgramIdA,
            },
            signers: [newEscrow],
          }
        );

        let _initializerTokenAccountA = await getAccount(
          connection,
          initializerTokenAccountA,
          undefined,
          tokenProgramIdA
        );

        // Check that the new owner is the PDA.
        assert.isTrue(_initializerTokenAccountA.owner.equals(pda));

        // Cancel the escrow.
        await program.rpc.cancelEscrow({
          accounts: {
            initializer: provider.wallet.publicKey,
            pdaDepositTokenAccount: initializerTokenAccountA,
            pdaAccount: pda,
            escrowAccount: newEscrow.publicKey,
            tokenProgram: tokenProgramIdA,
          },
        });

        // Check the final owner should be the provider public key.
        _initializerTokenAccountA = await getAccount(
          connection,
          initializerTokenAccountA,
          undefined,
          tokenProgramIdA
        );
        assert.isTrue(
          _initializerTokenAccountA.owner.equals(provider.wallet.publicKey)
        );

        // Check all the funds are still there.
        assert.strictEqual(
          _initializerTokenAccountA.amount,
          BigInt(initializerAmount)
        );
      });
    });
  });
});
