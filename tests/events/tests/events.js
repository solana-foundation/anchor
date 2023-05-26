const anchor = require("@coral-xyz/anchor");
const { assert } = require("chai");

describe("Events", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Events;

  describe("Normal event", () => {
    it("Single event works", async () => {
      let listener = null;

      let [event, slot] = await new Promise((resolve, _reject) => {
        listener = program.addEventListener("MyEvent", (event, slot) => {
          resolve([event, slot]);
        });
        program.rpc.initialize();
      });
      await program.removeEventListener(listener);

      assert.isAbove(slot, 0);
      assert.strictEqual(event.data.toNumber(), 5);
      assert.strictEqual(event.label, "hello");
    });

    it("Multiple events work", async () => {
      let listenerOne = null;
      let listenerTwo = null;

      let [eventOne, slotOne] = await new Promise((resolve, _reject) => {
        listenerOne = program.addEventListener("MyEvent", (event, slot) => {
          resolve([event, slot]);
        });
        program.rpc.initialize();
      });

      let [eventTwo, slotTwo] = await new Promise((resolve, _reject) => {
        listenerTwo = program.addEventListener(
          "MyOtherEvent",
          (event, slot) => {
            resolve([event, slot]);
          }
        );
        program.rpc.testEvent();
      });

      await program.removeEventListener(listenerOne);
      await program.removeEventListener(listenerTwo);

      assert.isAbove(slotOne, 0);
      assert.strictEqual(eventOne.data.toNumber(), 5);
      assert.strictEqual(eventOne.label, "hello");

      assert.isAbove(slotTwo, 0);
      assert.strictEqual(eventTwo.data.toNumber(), 6);
      assert.strictEqual(eventTwo.label, "bye");
    });
  });

  describe("Self-CPI event", () => {
    it("Works without accounts being specified", async () => {
      const tx = await program.methods.testEventCpi().transaction();
      const config = {
        commitment: "confirmed",
      };
      const txHash = await program.provider.sendAndConfirm(tx, [], config);
      const txResult = await program.provider.connection.getTransaction(
        txHash,
        config
      );

      const ixData = anchor.utils.bytes.bs58.decode(
        txResult.meta.innerInstructions[0].instructions[0].data
      );
      const eventData = anchor.utils.bytes.base64.encode(ixData.slice(8));
      const event = program.coder.events.decode(eventData);

      assert.strictEqual(event.name, "MyOtherEvent");
      assert.strictEqual(event.data.label, "cpi");
      assert.strictEqual(event.data.data.toNumber(), 7);
    });

    it("Malicious invocation throws", async () => {
      const tx = new anchor.web3.Transaction();
      tx.add(
        new anchor.web3.TransactionInstruction({
          programId: program.programId,
          keys: [
            {
              pubkey: anchor.web3.PublicKey.findProgramAddressSync(
                [Buffer.from("__event_authority")],
                program.programId
              )[0],
              isSigner: false,
              isWritable: false,
            },
            {
              pubkey: program.programId,
              isSigner: false,
              isWritable: false,
            },
          ],
          data: Buffer.from([0xe4, 0x45, 0xa5, 0x2e, 0x51, 0xcb, 0x9a, 0x1d]),
        })
      );

      try {
        await program.provider.sendAndConfirm(tx, []);
      } catch (e) {
        if (e.logs.some((log) => log.includes("ConstraintSigner"))) return;
        console.log(e);
      }

      throw new Error("Was able to invoke the self-CPI instruction");
    });
  });
});
