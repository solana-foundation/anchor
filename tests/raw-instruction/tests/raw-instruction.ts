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

  it("Rejects empty buffer", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(0);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    const emptyBuffer = Buffer.alloc(0);

    try {
      await (program.methods as any)
        [methodName](emptyBuffer)
        .accounts({
          dataAccount: dataAccount.publicKey,
        })
        .rpc();
      expect.fail("Should have thrown an error");
    } catch (err: any) {
      expect(err.toString()).to.include("InvalidDataLength");
    }
  });

  it("Rejects buffer shorter than 8 bytes", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(0);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    const shortBuffer = Buffer.from([1, 2, 3, 4, 5, 6, 7]); // Only 7 bytes

    try {
      await (program.methods as any)
        [methodName](shortBuffer)
        .accounts({
          dataAccount: dataAccount.publicKey,
        })
        .rpc();
      expect.fail("Should have thrown an error");
    } catch (err: any) {
      expect(err.toString()).to.include("InvalidDataLength");
    }
  });

  it("Handles exactly 8 bytes", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(0);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    const value = new BN(12345);
    const exactBytes = Buffer.alloc(8);
    value.toArrayLike(Buffer, "le", 8).copy(exactBytes);

    await (program.methods as any)
      [methodName](exactBytes)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .rpc();

    const account = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    expect(account.data.toNumber()).to.equal(12345);
  });

  it("Handles buffer longer than 8 bytes (uses first 8)", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(0);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    const value = new BN(99999);
    const longBuffer = Buffer.alloc(16);
    value.toArrayLike(Buffer, "le", 8).copy(longBuffer);
    // Fill rest with different values
    longBuffer.writeUInt32LE(0xdeadbeef, 8);
    longBuffer.writeUInt32LE(0xcafebabe, 12);

    await (program.methods as any)
      [methodName](longBuffer)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .rpc();

    const account = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    expect(account.data.toNumber()).to.equal(99999);
  });

  it("Handles maximum u64 value", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(0);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    const maxValue = new BN("18446744073709551615"); // 2^64 - 1
    const maxBytes = Buffer.alloc(8);
    maxValue.toArrayLike(Buffer, "le", 8).copy(maxBytes);

    await (program.methods as any)
      [methodName](maxBytes)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .rpc();

    const account = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    expect(account.data.toString()).to.equal("18446744073709551615");
  });

  it("Handles zero value", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(100);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    const zeroBytes = Buffer.alloc(8); // All zeros

    await (program.methods as any)
      [methodName](zeroBytes)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .rpc();

    const account = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    expect(account.data.toNumber()).to.equal(0);
  });

  it("Handles multiple consecutive raw instructions", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(0);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    const values = [100, 200, 300, 400, 500];
    for (const value of values) {
      const rawBytes = Buffer.alloc(8);
      new BN(value).toArrayLike(Buffer, "le", 8).copy(rawBytes);

      await (program.methods as any)
        [methodName](rawBytes)
        .accounts({
          dataAccount: dataAccount.publicKey,
        })
        .rpc();
    }

    const account = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    expect(account.data.toNumber()).to.equal(500); // Last value
  });

  it("Handles various byte patterns", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(0);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    // Test with alternating pattern
    const pattern1 = Buffer.from([
      0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22,
    ]);
    await (program.methods as any)
      [methodName](pattern1)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .rpc();

    const account1 = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    // Read as little-endian u64 (same as Rust does)
    const low = pattern1.readUInt32LE(0);
    const high = pattern1.readUInt32LE(4);
    const expected1 = new BN(low).add(new BN(high).mul(new BN(0x100000000)));
    expect(account1.data.toString()).to.equal(expected1.toString());

    // Test with all ones
    const pattern2 = Buffer.from([
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    ]);
    await (program.methods as any)
      [methodName](pattern2)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .rpc();

    const account2 = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    expect(account2.data.toString()).to.equal("18446744073709551615");
  });

  it("Verifies raw bytes are not Borsh-encoded", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(0);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    const value = new BN(42);
    const rawBytes = Buffer.alloc(8);
    value.toArrayLike(Buffer, "le", 8).copy(rawBytes);

    // Build instruction and inspect the data
    const instruction = await (program.methods as any)
      [methodName](rawBytes)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .instruction();

    const dataAfterDiscriminator = instruction.data.slice(8);

    expect(dataAfterDiscriminator.length).to.equal(8);
    expect(dataAfterDiscriminator.equals(rawBytes)).to.be.true;
    expect(dataAfterDiscriminator[0]).to.equal(42);
  });

  it("Handles Uint8Array input", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialValue = new BN(0);

    await program.methods
      .initialize(initialValue)
      .accounts({
        dataAccount: dataAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const methodName =
      program.idl.instructions.find(
        (ix) => ix.name === "rawHandler" || ix.name === "raw_handler"
      )?.name || "rawHandler";

    const value = new BN(777);
    const rawBytes = Buffer.alloc(8);
    value.toArrayLike(Buffer, "le", 8).copy(rawBytes);

    // Convert to Uint8Array
    const uint8Array = new Uint8Array(rawBytes);

    await (program.methods as any)
      [methodName](uint8Array)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .rpc();

    const account = await program.account.dataAccount.fetch(
      dataAccount.publicKey
    );
    expect(account.data.toNumber()).to.equal(777);
  });
});
