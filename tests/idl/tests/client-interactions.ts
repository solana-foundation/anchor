import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";

import { ClientInteractions } from "../target/types/client_interactions";

describe("Client interactions", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .clientInteractions as anchor.Program<ClientInteractions>;

  it("Can use integers", async () => {
    const kp = anchor.web3.Keypair.generate();

    const i8 = -3;
    const i16 = 1;
    const i32 = -5555551;
    const i64 = new anchor.BN("384535471");
    const i128 = new anchor.BN(-8342491);

    await program.methods
      .int(i8, i16, i32, i64, i128)
      .accounts({ account: kp.publicKey })
      .signers([kp])
      .preInstructions([await program.account.intAccount.createInstruction(kp)])
      .rpc();

    const account = await program.account.intAccount.fetch(kp.publicKey);
    assert.strictEqual(account.i8, i8);
    assert.strictEqual(account.i16, i16);
    assert.strictEqual(account.i32, i32);
    assert(account.i64.eq(i64));
    assert(account.i128.eq(i128));
  });

  it("Can use unsigned integers", async () => {
    const kp = anchor.web3.Keypair.generate();

    const u8 = 123;
    const u16 = 7888;
    const u32 = 5555551;
    const u64 = new anchor.BN("384535471");
    const u128 = new anchor.BN(8888888);

    await program.methods
      .uint(u8, u16, u32, u64, u128)
      .accounts({ account: kp.publicKey })
      .signers([kp])
      .preInstructions([
        await program.account.unsignedIntAccount.createInstruction(kp),
      ])
      .rpc();

    const account = await program.account.unsignedIntAccount.fetch(
      kp.publicKey
    );
    assert.strictEqual(account.u8, u8);
    assert.strictEqual(account.u16, u16);
    assert.strictEqual(account.u32, u32);
    assert(account.u64.eq(u64));
    assert(account.u128.eq(u128));
  });

  it("Can use enum", async () => {
    const testAccountEnum = async (
      ...args: Parameters<typeof program["methods"]["enm"]>
    ) => {
      const kp = anchor.web3.Keypair.generate();
      await program.methods
        .enm(...(args as any))
        .accounts({ account: kp.publicKey })
        .signers([kp])
        .preInstructions([
          await program.account.enumAccount.createInstruction(kp),
        ])
        .rpc();
      return await program.account.enumAccount.fetch(kp.publicKey);
    };

    // Unit
    const unit = await testAccountEnum({ unit: {} });
    assert.deepEqual(unit.enumField.unit, {});

    // Named
    const pointX = new anchor.BN(1);
    const pointY = new anchor.BN(2);
    const named = await testAccountEnum({ named: { pointX, pointY } });
    assert(named.enumField.named.pointX.eq(pointX));
    assert(named.enumField.named.pointY.eq(pointY));

    // Unnamed
    const tupleArg = [1, 2, 3, 4] as const;
    const unnamed = await testAccountEnum({ unnamed: tupleArg });
    assert.strictEqual(unnamed.enumField.unnamed[0], tupleArg[0]);
    assert.strictEqual(unnamed.enumField.unnamed[1], tupleArg[1]);
    assert.strictEqual(unnamed.enumField.unnamed[2], tupleArg[2]);
    assert.strictEqual(unnamed.enumField.unnamed[3], tupleArg[3]);

    // Unnamed struct
    const tupleStructArg = [
      { u8: 1, u16: 11, u32: 111, u64: new anchor.BN(1111) },
    ] as const;
    const unnamedStruct = await testAccountEnum({
      unnamedStruct: tupleStructArg,
    });
    assert.strictEqual(
      unnamedStruct.enumField.unnamedStruct[0].u8,
      tupleStructArg[0].u8
    );
    assert.strictEqual(
      unnamedStruct.enumField.unnamedStruct[0].u16,
      tupleStructArg[0].u16
    );
    assert.strictEqual(
      unnamedStruct.enumField.unnamedStruct[0].u32,
      tupleStructArg[0].u32
    );
    assert(
      unnamedStruct.enumField.unnamedStruct[0].u64.eq(tupleStructArg[0].u64)
    );
  });

  it("Can use type aliases", async () => {
    const kp = anchor.web3.Keypair.generate();

    const typeAliasU8 = 42;
    const typeAliasU8Array = [1, 2, 3, 4, 5, 6, 7, 8];
    const typeAliasStruct = {
      u8: 1,
      u16: 2,
      u32: 3,
      u64: new anchor.BN(4),
    };

    await program.methods
      .typeAlias(typeAliasU8, typeAliasU8Array, typeAliasStruct)
      .accounts({ account: kp.publicKey })
      .signers([kp])
      .preInstructions([await program.account.intAccount.createInstruction(kp)])
      .rpc();

    const account = await program.account.typeAliasAccount.fetch(kp.publicKey);
    assert.strictEqual(account.typeAliasU8, typeAliasU8);
    assert.deepEqual(account.typeAliasU8Array, typeAliasU8Array);
    assert.strictEqual(account.typeAliasStruct.u8, typeAliasStruct.u8);
    assert.strictEqual(account.typeAliasStruct.u16, typeAliasStruct.u16);
    assert.strictEqual(account.typeAliasStruct.u32, typeAliasStruct.u32);
    assert(account.typeAliasStruct.u64.eq(typeAliasStruct.u64));
  });
});
