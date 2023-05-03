import * as anchor from "@coral-xyz/anchor";
import * as token from "@coral-xyz/spl-token";
import { spawnSync } from "child_process";

import { Bench, IDL } from "../target/types/bench";
import { BenchData, ComputeUnits, getVersionFromArgs } from "../scripts/utils";

describe(IDL.name, () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Bench as anchor.Program<Bench>;
  const owner = program.provider.publicKey!;

  let mintPk: anchor.web3.PublicKey;
  let tokenPk: anchor.web3.PublicKey;

  const computeUnits: ComputeUnits = {};

  const measureComputeUnits = async (
    ixName: string,
    options?: Partial<{
      accountCounts: number[];
      generateKeypair: (accountName: string) => anchor.web3.Keypair;
      generatePublicKey: (accountName: string) => anchor.web3.PublicKey;
    }>
  ) => {
    options ??= {};
    options.accountCounts ??= [1, 2, 4, 8];
    options.generateKeypair ??= () => anchor.web3.Keypair.generate();

    for (const accountCount of options.accountCounts) {
      // Check whether the init version of the instruction exists
      const ixNameInit = `${ixName}Init`;
      const hasInitVersion = IDL.instructions.some((ix) =>
        ix.name.startsWith(ixNameInit)
      );

      const ixNames = [ixName];
      if (hasInitVersion) {
        // Init version has priority
        ixNames.unshift(ixNameInit);
      }

      const accounts: { [key: string]: anchor.web3.PublicKey } = {};
      const signers = [];

      for (const ixName of ixNames) {
        const method =
          `${ixName}${accountCount}` as keyof typeof program.methods;

        // Remove signers  when it's not init instruction
        if (ixName !== ixNameInit) {
          signers.splice(0);
        }

        for (const ix of IDL.instructions) {
          if (ix.name !== method) continue;

          for (const account of ix.accounts) {
            // Only set account keys if it hasn't been set before
            if (accounts[account.name]) {
              continue;
            }

            if (account.name === "payer") {
              accounts[account.name] = owner;
              continue;
            }

            // Skip other accounts to not override Anchor defaults
            if (!account.name.startsWith("account")) {
              continue;
            }

            if (options.generatePublicKey) {
              accounts[account.name] = options.generatePublicKey(account.name);
              continue;
            }

            const keypair = options.generateKeypair(account.name);
            accounts[account.name] = keypair.publicKey;

            if (account.isSigner) {
              signers.push(keypair);
            }
          }
        }

        // Send tx
        console.log({ method });
        const txHash = await program.methods[method]()
          .accounts(accounts)
          .signers(signers)
          .rpc();

        // Confirm tx
        await program.provider.connection.confirmTransaction(
          txHash,
          "confirmed"
        );

        // Get tx
        const tx = await program.provider.connection.getTransaction(txHash, {
          commitment: "confirmed",
        });

        computeUnits[method] = tx!.meta!.computeUnitsConsumed!;
      }
    }
  };

  before(async () => {
    const tokenProgram = token.splTokenProgram({
      provider: anchor.AnchorProvider.local(),
    });

    const tx = new anchor.web3.Transaction();

    // Create mint account
    const mintKp = new anchor.web3.Keypair();
    mintPk = mintKp.publicKey;
    const createMintIx = await tokenProgram.account.mint.createInstruction(
      mintKp
    );
    const initMintIx = await tokenProgram.methods
      .initializeMint2(0, owner, null)
      .accounts({ mint: mintPk })
      .instruction();
    tx.add(createMintIx, initMintIx);

    // Create token account
    const tokenKp = new anchor.web3.Keypair();
    tokenPk = tokenKp.publicKey;
    const createTokenIx = await tokenProgram.account.account.createInstruction(
      tokenKp
    );
    const initTokenIx = await tokenProgram.methods
      .initializeAccount3(owner)
      .accounts({ account: tokenPk, mint: mintPk })
      .instruction();
    tx.add(createTokenIx, initTokenIx);

    await tokenProgram.provider.sendAndConfirm!(tx, [mintKp, tokenKp]);
  });

  it("AccountInfo", async () => {
    await measureComputeUnits("accountInfo");
  });

  it("Account Empty", async () => {
    await measureComputeUnits("accountEmpty");
  });

  it("Account Sized", async () => {
    await measureComputeUnits("accountSized");
  });

  it("Account Unsized", async () => {
    await measureComputeUnits("accountUnsized");
  });

  it("Boxed Account Empty", async () => {
    await measureComputeUnits("boxedAccountEmpty");
  });

  it("Boxed Account Sized", async () => {
    await measureComputeUnits("boxedAccountSized");
  });

  it("Boxed Account Unsized", async () => {
    await measureComputeUnits("boxedAccountUnsized");
  });

  it("Boxed Interface Account Mint", async () => {
    await measureComputeUnits("boxedInterfaceAccountMint", {
      generatePublicKey: () => mintPk,
    });
  });

  it("Boxed Interface Account Token", async () => {
    await measureComputeUnits("boxedInterfaceAccountToken", {
      generatePublicKey: () => tokenPk,
    });
  });

  it("Interface Account Mint", async () => {
    await measureComputeUnits("interfaceAccountMint", {
      generatePublicKey: () => mintPk,
    });
  });

  it("Interface Account Token", async () => {
    await measureComputeUnits("interfaceAccountToken", {
      generatePublicKey: () => tokenPk,
      accountCounts: [1, 2, 4],
    });
  });

  it("Interface", async () => {
    await measureComputeUnits("interface", {
      generatePublicKey: () => token.SPL_TOKEN_PROGRAM_ID,
    });
  });

  it("Program", async () => {
    await measureComputeUnits("program", {
      generatePublicKey: () => anchor.web3.SystemProgram.programId,
    });
  });

  it("Signer", async () => {
    await measureComputeUnits("signer");
  });

  it("SystemAccount", async () => {
    await measureComputeUnits("systemAccount");
  });

  it("UncheckedAccount", async () => {
    await measureComputeUnits("uncheckedAccount");
  });

  after(async () => {
    // Read the bench data file
    const bench = await BenchData.open();

    // Compare and update compute units changes
    const version = getVersionFromArgs();
    const oldComputeUnits = bench.get(version).computeUnits;
    const { needsUpdate } = bench.compareComputeUnits(
      computeUnits,
      oldComputeUnits,
      ({ ixName, newComputeUnits: newValue }) => {
        if (newValue === null) {
          delete oldComputeUnits[ixName];
        } else {
          oldComputeUnits[ixName] = newValue;
        }
      }
    );

    if (needsUpdate) {
      console.log("Updating benchmark files...");

      // Save bench data file
      // (needs to happen before running the `sync-markdown` script)
      await bench.save();

      // Only update markdown files on `unreleased` version
      if (version === "unreleased") {
        spawnSync("anchor", ["run", "sync-markdown"]);
      }
    }
  });
});
