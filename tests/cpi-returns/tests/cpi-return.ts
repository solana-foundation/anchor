import assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as borsh from "borsh";
import { Program } from "@project-serum/anchor";
import { Callee } from "../target/types/callee";
import { Caller } from "../target/types/caller";

const { SystemProgram } = anchor.web3;

describe("CPI return", () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const callerProgram = anchor.workspace.Caller as Program<Caller>;
  const calleeProgram = anchor.workspace.Callee as Program<Callee>;

  const getReturnLog = (confirmedTransaction) => {
    const prefix = "Program return: ";
    let log = confirmedTransaction.meta.logMessages.find((log) =>
      log.startsWith(prefix)
    );
    log = log.slice(prefix.length);
    const [key, data] = log.split(" ", 2);
    const buffer = Buffer.from(data, "base64");
    return [key, data, buffer];
  };

  const cpiReturn = anchor.web3.Keypair.generate();

  const confirmOptions = { commitment: "confirmed" };

  it("can initialize", async () => {
    await calleeProgram.methods
      .initialize()
      .accounts({
        account: cpiReturn.publicKey,
        user: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([cpiReturn])
      .rpc();
  });

  it("can return u64 from a cpi", async () => {
    const tx = await callerProgram.methods
      .cpiCallReturnU64()
      .accounts({
        cpiReturn: cpiReturn.publicKey,
        cpiReturnProgram: calleeProgram.programId,
      })
      .rpc(confirmOptions);
    let t = await provider.connection.getTransaction(tx, {
      commitment: "confirmed",
    });

    const [key, data, buffer] = getReturnLog(t);
    assert.equal(key, calleeProgram.programId);

    // Check for matching log on receive side
    let receiveLog = t.meta.logMessages.find(
      (log) => log == `Program data: ${data}`
    );
    assert(receiveLog !== undefined);

    const reader = new borsh.BinaryReader(buffer);
    assert.equal(reader.readU64().toNumber(), 10);
  });

  it("can make a non-cpi call to a function that returns a u64", async () => {
    const tx = await calleeProgram.methods
      .returnU64()
      .accounts({
        account: cpiReturn.publicKey,
      })
      .rpc(confirmOptions);
    let t = await provider.connection.getTransaction(tx, {
      commitment: "confirmed",
    });
    const [key, , buffer] = getReturnLog(t);
    assert.equal(key, calleeProgram.programId);
    const reader = new borsh.BinaryReader(buffer);
    assert.equal(reader.readU64().toNumber(), 10);
  });

  it("can return a struct from a cpi", async () => {
    const tx = await callerProgram.methods
      .cpiCallReturnStruct()
      .accounts({
        cpiReturn: cpiReturn.publicKey,
        cpiReturnProgram: calleeProgram.programId,
      })
      .rpc(confirmOptions);
    let t = await provider.connection.getTransaction(tx, {
      commitment: "confirmed",
    });

    const [key, data, buffer] = getReturnLog(t);
    assert.equal(key, calleeProgram.programId);

    // Check for matching log on receive side
    let receiveLog = t.meta.logMessages.find(
      (log) => log == `Program data: ${data}`
    );
    assert(receiveLog !== undefined);

    // Deserialize the struct and validate
    class Assignable {
      constructor(properties) {
        Object.keys(properties).map((key) => {
          this[key] = properties[key];
        });
      }
    }
    class Data extends Assignable {}
    const schema = new Map([
      [Data, { kind: "struct", fields: [["value", "u64"]] }],
    ]);
    const deserialized = borsh.deserialize(schema, Data, buffer);
    assert(deserialized.value.toNumber() === 11);
  });

  it("can return a vec from a cpi", async () => {
    const tx = await callerProgram.methods
      .cpiCallReturnVec()
      .accounts({
        cpiReturn: cpiReturn.publicKey,
        cpiReturnProgram: calleeProgram.programId,
      })
      .rpc(confirmOptions);
    let t = await provider.connection.getTransaction(tx, {
      commitment: "confirmed",
    });

    const [key, data, buffer] = getReturnLog(t);
    assert.equal(key, calleeProgram.programId);

    // Check for matching log on receive side
    let receiveLog = t.meta.logMessages.find(
      (log) => log == `Program data: ${data}`
    );
    assert(receiveLog !== undefined);

    const reader = new borsh.BinaryReader(buffer);
    const array = reader.readArray(() => reader.readU8());
    assert.deepStrictEqual(array, [12, 13, 14, 100]);
  });

  it("sets a return value in idl", async () => {
    const returnu64Instruction = calleeProgram._idl.instructions.find(
      (f) => f.name == "returnU64"
    );
    assert.equal(returnu64Instruction.returns, "u64");

    const returnStructInstruction = calleeProgram._idl.instructions.find(
      (f) => f.name == "returnStruct"
    );
    assert.deepStrictEqual(returnStructInstruction.returns, {
      defined: "StructReturn",
    });
  });
});
