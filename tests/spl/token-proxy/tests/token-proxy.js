const { PublicKey, Keypair } = require("@solana/web3.js");
const anchor = require("@coral-xyz/anchor");
const { BN } = require("@coral-xyz/anchor");
const { assert } = require("chai");

const {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  createMint,
  createAccount,
  getAccount,
  getMint,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} = require("@solana/spl-token");

describe("program", () => {
  const provider = anchor.AnchorProvider.local();

  const TEST_PROGRAM_IDS = [TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID];

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);
  const connection = provider.connection;
  const payer = provider.wallet.payer;

  const program = anchor.workspace.TokenProxy;
  /**
   * create a mint by tokenProgramId for test.
   * @param {*} tokenProgramId
   * @returns pub key of the mint
   */
  const createMintByTokenProgramId = async (tokenProgramId) => {
    return createMint(
      connection,
      payer,
      provider.wallet.publicKey,
      undefined,
      0,
      Keypair.generate(),
      undefined,
      tokenProgramId
    );
  };

  TEST_PROGRAM_IDS.forEach((tokenProgramId) => {
    const name = tokenProgramId.equals(TOKEN_2022_PROGRAM_ID)
      ? "token"
      : "token-2022";

    describe(name, () => {
      let mint = null;
      let from = null;
      let to = null;

      it("Initializes test state", async () => {
        mint = await createMintByTokenProgramId(tokenProgramId);

        from = await createAccount(
          connection,
          payer,
          mint,
          provider.wallet.publicKey,
          Keypair.generate(),
          undefined,
          tokenProgramId
        );

        to = await createAccount(
          connection,
          payer,
          mint,
          provider.wallet.publicKey,
          Keypair.generate(),
          // {commitment: "confirmed"},
          undefined,
          tokenProgramId
        );
      });

      it("Creates a token account", async () => {
        const newMint = await createMintByTokenProgramId(tokenProgramId);

        const authority = provider.wallet.publicKey;
        const [tokenAccount] = PublicKey.findProgramAddressSync(
          [
            authority.toBytes(),
            newMint.toBytes(),
            Buffer.from("token-proxy-account"),
          ],
          program.programId
        );
        await program.rpc.proxyCreateTokenAccount({
          accounts: {
            authority,
            mint: newMint,
            tokenAccount,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: tokenProgramId,
          },
        });
        const account = await getAccount(
          connection,
          tokenAccount,
          null,
          tokenProgramId
        );
        assert.isTrue(account.amount === BigInt(0));
      });

      it("Creates an associated token account", async () => {
        const newMint = await createMintByTokenProgramId(tokenProgramId);
        const authority = provider.wallet.publicKey;

        const [tokenAccount] = PublicKey.findProgramAddressSync(
          [authority.toBytes(), tokenProgramId.toBytes(), newMint.toBytes()],
          ASSOCIATED_TOKEN_PROGRAM_ID
        );

        await program.rpc.proxyCreateAssociatedTokenAccount({
          accounts: {
            tokenAccount,
            mint: newMint,
            authority,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: tokenProgramId,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          },
        });
        const account = await getAccount(
          connection,
          tokenAccount,
          undefined,
          tokenProgramId
        );
        assert.isTrue(account.amount === BigInt(0));
      });

      it("Creates a mint", async () => {
        const authority = provider.wallet.publicKey;
        const [newMint] = PublicKey.findProgramAddressSync(
          [
            authority.toBytes(),
            Buffer.from(name),
            Buffer.from("token-proxy-mint"),
          ],
          program.programId
        );
        await program.rpc.proxyCreateMint(name, {
          accounts: {
            authority,
            mint: newMint,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: tokenProgramId,
          },
        });
      });

      it("Mints a token", async () => {
        await program.rpc.proxyMintTo(new anchor.BN(1000), {
          accounts: {
            authority: provider.wallet.publicKey,
            mint,
            to: from,
            tokenProgram: tokenProgramId,
          },
        });

        const fromAccount = await getAccount(
          connection,
          from,
          undefined,
          tokenProgramId
        );

        assert.isTrue(fromAccount.amount === BigInt(1000));
      });

      it("Transfers a token", async () => {
        const preFromAccount = await getAccount(
          connection,
          from,
          undefined,
          tokenProgramId
        );
        const preToAccount = await getAccount(
          connection,
          to,
          undefined,
          tokenProgramId
        );

        const transferAmount = new BN(400);

        await program.rpc.proxyTransfer(transferAmount, {
          accounts: {
            authority: provider.wallet.publicKey,
            to,
            from,
            tokenProgram: tokenProgramId,
          },
        });

        const postFromAccount = await getAccount(
          connection,
          from,
          undefined,
          tokenProgramId
        );
        const postToAccount = await getAccount(
          connection,
          to,
          undefined,
          tokenProgramId
        );

        assert.isTrue(
          postFromAccount.amount ===
            preFromAccount.amount - BigInt(transferAmount)
        );
        assert.isTrue(
          postToAccount.amount === preToAccount.amount + BigInt(transferAmount)
        );
      });

      it("Transfers a token with optional accounts", async () => {
        const preFromAccount = await getAccount(
          connection,
          from,
          undefined,
          tokenProgramId
        );
        const preToAccount = await getAccount(
          connection,
          to,
          undefined,
          tokenProgramId
        );

        const transferAmount = 10;

        await program.rpc.proxyOptionalTransfer(new BN(transferAmount), {
          accounts: {
            authority: provider.wallet.publicKey,
            to,
            from,
            mint,
            tokenProgram: tokenProgramId,
          },
        });

        const postFromAccount = await getAccount(
          connection,
          from,
          undefined,
          tokenProgramId
        );
        const postToAccount = await getAccount(
          connection,
          to,
          undefined,
          tokenProgramId
        );

        assert.isTrue(
          postFromAccount.amount ===
            preFromAccount.amount - BigInt(transferAmount)
        );
        assert.isTrue(
          postToAccount.amount === preToAccount.amount + BigInt(transferAmount)
        );
      });

      it("Does not transfer a token without optional accounts", async () => {
        const preFromAccount = await getAccount(
          connection,
          from,
          undefined,
          tokenProgramId
        );
        const preToAccount = await getAccount(
          connection,
          to,
          undefined,
          tokenProgramId
        );

        const optionalTransferIx = await program.methods
          .proxyOptionalTransfer(new anchor.BN(10))
          .accounts({
            authority: provider.wallet.publicKey,
            to,
            from,
            mint: null,
            tokenProgram: null,
          })
          .instruction();
        const tx = new anchor.web3.Transaction().add(optionalTransferIx);
        await provider.sendAndConfirm(tx);

        const postFromAccount = await getAccount(
          connection,
          from,
          undefined,
          tokenProgramId
        );
        const postToAccount = await getAccount(
          connection,
          to,
          undefined,
          tokenProgramId
        );

        assert.isTrue(postFromAccount.amount === preFromAccount.amount);
        assert.isTrue(postToAccount.amount === preToAccount.amount);
      });

      it("Burns a token", async () => {
        const preAccount = await getAccount(
          connection,
          to,
          undefined,
          tokenProgramId
        );
        const burnAmount = 300;
        await program.rpc.proxyBurn(new BN(burnAmount), {
          accounts: {
            authority: provider.wallet.publicKey,
            mint,
            from: to,
            tokenProgram: tokenProgramId,
          },
        });

        const postAccount = await getAccount(
          connection,
          to,
          undefined,
          tokenProgramId
        );
        assert.isTrue(
          postAccount.amount === preAccount.amount - BigInt(burnAmount)
        );
      });

      it("Set new mint authority", async () => {
        const newMintAuthority = Keypair.generate();
        await program.rpc.proxySetAuthority(
          { mintTokens: {} },
          newMintAuthority.publicKey,
          {
            accounts: {
              accountOrMint: mint,
              currentAuthority: provider.wallet.publicKey,
              tokenProgram: tokenProgramId,
            },
          }
        );

        const mintInfo = await getMint(
          connection,
          mint,
          undefined,
          tokenProgramId
        );
        assert.isTrue(
          mintInfo.mintAuthority.equals(newMintAuthority.publicKey)
        );
      });
    });
  });
});
