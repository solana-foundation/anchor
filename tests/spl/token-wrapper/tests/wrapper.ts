import * as anchor from "@coral-xyz/anchor";
import { Program, BN, IdlAccounts } from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  createMint,
  createAccount,
  getAccount,
  mintTo,
} from "@solana/spl-token";
import { assert } from "chai";
import { TokenWrapper } from "../target/types/token_wrapper";

describe("wrapper", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const connection = provider.connection;

  const TEST_TOKEN_PROGRAM_IDS = [
    [TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID],
    [TOKEN_2022_PROGRAM_ID, TOKEN_2022_PROGRAM_ID],
    [TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID],
    [TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID],
  ];
  const program = anchor.workspace.TokenWrapper as Program<TokenWrapper>;

  let depositMint: PublicKey = null;
  let wrappedMintKP: Keypair = null;
  let initializerDepositTokenAccount: PublicKey = null;
  let userWrappedTokenAccount: PublicKey = null;
  let userDepositTokenAccount: PublicKey = null;
  let depositTokenVault: PublicKey = null;
  let wrapperAuthority: PublicKey = null;

  const wrapAmount = 1000;
  const initializerAmount = 500;

  const payer = Keypair.generate();
  const mintAuthority = Keypair.generate();

  TEST_TOKEN_PROGRAM_IDS.forEach((tokenProgramIds) => {
    const [depositTokenProgram, wrappedTokenProgram] = tokenProgramIds;
    let name;
    if (depositTokenProgram === wrappedTokenProgram) {
      name = wrappedTokenProgram === TOKEN_PROGRAM_ID ? "token" : "token-2022";
    } else {
      name =
        wrappedTokenProgram === TOKEN_PROGRAM_ID
          ? "mixed-wrapped-token"
          : "mixed-wrapped-token-2022";
    }
    describe(name, () => {
      it("Initialise wrapper", async () => {
        // Airdropping tokens to a payer.
        await connection.confirmTransaction(
          await connection.requestAirdrop(
            payer.publicKey,
            10 * LAMPORTS_PER_SOL
          ),
          "confirmed"
        );

        depositMint = await createMint(
          connection,
          payer,
          mintAuthority.publicKey,
          null,
          6,
          undefined,
          undefined,
          depositTokenProgram
        );
        wrappedMintKP = new Keypair();

        initializerDepositTokenAccount = await createAccount(
          connection,
          payer,
          depositMint,
          provider.wallet.publicKey,
          Keypair.generate(),
          undefined,
          depositTokenProgram
        );

        await mintTo(
          connection,
          payer,
          depositMint,
          initializerDepositTokenAccount,
          mintAuthority.publicKey,
          initializerAmount,
          [mintAuthority],
          undefined,
          depositTokenProgram
        );

        const initializerWrappedTokenAccountKP = new Keypair();

        // Get the PDA that is assigned authority to the deposit vault account
        const [_wrapperAuthority, _] = PublicKey.findProgramAddressSync(
          [
            Buffer.from(anchor.utils.bytes.utf8.encode("wrapr")),
            depositMint.toBuffer(),
            wrappedMintKP.publicKey.toBuffer(),
          ],
          program.programId
        );
        wrapperAuthority = _wrapperAuthority;

        // Get the deposit vault account PDA
        const [_depositTokenVault, __] = PublicKey.findProgramAddressSync(
          [
            Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
            depositMint.toBuffer(),
            wrappedMintKP.publicKey.toBuffer(),
          ],
          program.programId
        );
        depositTokenVault = _depositTokenVault;

        await program.rpc.initialize(new BN(initializerAmount), {
          accounts: {
            initializer: provider.wallet.publicKey,
            depositMint: depositMint,
            wrappedMint: wrappedMintKP.publicKey,
            depositTokenVault,
            initializerDepositTokenAccount,
            initializerWrappedTokenAccount:
              initializerWrappedTokenAccountKP.publicKey,
            wrapperAuthority,
            systemProgram: SystemProgram.programId,
            depositTokenProgram,
            wrappedTokenProgram,
          },
          signers: [wrappedMintKP, initializerWrappedTokenAccountKP],
        });

        let _initializerDepositTokenAccount = await getAccount(
          connection,
          initializerDepositTokenAccount,
          undefined,
          depositTokenProgram
        );
        let _initializerWrappedTokenAccount = await getAccount(
          connection,
          initializerWrappedTokenAccountKP.publicKey,
          undefined,
          wrappedTokenProgram
        );

        assert.strictEqual(_initializerDepositTokenAccount.amount, BigInt(0));
        assert.strictEqual(
          _initializerWrappedTokenAccount.amount,
          BigInt(initializerAmount)
        );
      });

      it("Wrap", async () => {
        userDepositTokenAccount = await createAccount(
          connection,
          payer,
          depositMint,
          provider.wallet.publicKey,
          Keypair.generate(),
          undefined,
          depositTokenProgram
        );
        userWrappedTokenAccount = await createAccount(
          connection,
          payer,
          wrappedMintKP.publicKey,
          provider.wallet.publicKey,
          Keypair.generate(),
          undefined,
          wrappedTokenProgram
        );

        await mintTo(
          connection,
          payer,
          depositMint,
          userDepositTokenAccount,
          mintAuthority.publicKey,
          wrapAmount,
          [mintAuthority],
          undefined,
          depositTokenProgram
        );

        await program.rpc.wrap(new BN(wrapAmount), {
          accounts: {
            signer: provider.wallet.publicKey,
            depositMint: depositMint,
            wrappedMint: wrappedMintKP.publicKey,
            depositTokenVault: depositTokenVault,
            userDepositTokenAccount,
            userWrappedTokenAccount,
            wrapperAuthority,
            depositTokenProgram,
            wrappedTokenProgram,
          },
        });

        let _userDepositTokenAccount = await getAccount(
          connection,
          userDepositTokenAccount,
          undefined,
          depositTokenProgram
        );
        let _userWrappedTokenAccount = await getAccount(
          connection,
          userWrappedTokenAccount,
          undefined,
          wrappedTokenProgram
        );

        assert.strictEqual(_userDepositTokenAccount.amount, BigInt(0));
        assert.strictEqual(_userWrappedTokenAccount.amount, BigInt(wrapAmount));
      });

      it("Unwrap", async () => {
        await program.rpc.unwrap(new BN(wrapAmount - 1), {
          accounts: {
            signer: provider.wallet.publicKey,
            depositMint: depositMint,
            wrappedMint: wrappedMintKP.publicKey,
            depositTokenVault: depositTokenVault,
            userDepositTokenAccount,
            userWrappedTokenAccount,
            wrapperAuthority,
            depositTokenProgram,
            wrappedTokenProgram,
          },
        });

        let _userDepositTokenAccount = await getAccount(
          connection,
          userDepositTokenAccount,
          undefined,
          depositTokenProgram
        );
        let _userWrappedTokenAccount = await getAccount(
          connection,
          userWrappedTokenAccount,
          undefined,
          wrappedTokenProgram
        );

        assert.strictEqual(
          _userDepositTokenAccount.amount,
          BigInt(wrapAmount - 1)
        );
        assert.strictEqual(_userWrappedTokenAccount.amount, BigInt(1));
      });
    });
  });
});
