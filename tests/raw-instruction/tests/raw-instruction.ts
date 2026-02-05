import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import type { RawInstruction } from "../target/types/raw_instruction";
import { expect } from "chai";
import { Buffer } from "buffer";
import BN from "bn.js";

describe("raw-instruction", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.RawInstruction as Program<RawInstruction>;

  it("Initializes data account", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(42);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const account = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    expect(account.data.toNumber()).to.equal(42);
  });

  it("Handles raw instruction with &[u8]", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(100);

    // Initialize first
    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    // Now test raw instruction
    const rawValue = new BN(200);
    const rawBytes = Buffer.alloc(8);
    rawValue.toArrayLike(Buffer, "le", 8).copy(rawBytes);

    // Note: method name is converted to snake_case in IDL
    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    await (program.methods as any)
      [methodName](rawBytes)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .rpc();

    const account = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    expect(account.data.toNumber()).to.equal(200);
  });

  it("Raw instruction appears in IDL", () => {
    const idl = program.idl;
    const rawHandlerIx = idl.instructions.find(
      (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
    );

    expect(rawHandlerIx).to.not.be.undefined;
    expect(rawHandlerIx!.args.length).to.equal(1);
    expect(rawHandlerIx!.args[0].name).to.equal("data");
    expect(rawHandlerIx!.args[0].type).to.equal("bytes");
  });
});
