const anchor = require("@project-serum/anchor");
const assert = require("assert");

describe("events", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const program = anchor.workspace.Events;

  it("Is initialized!", async () => {
    let listener = null;

    let [event, slot] = await new Promise((resolve, _reject) => {
      listener = program.addEventListener("MyEvent", (event, slot) => {
        resolve([event, slot]);
      });
      program.rpc.initialize();
    });
    await program.removeEventListener(listener);

    assert.ok(slot > 0);
    assert.ok(event.data.toNumber() === 5);
    assert.ok(event.label === "hello");
  });

  it("Multiple events", async () => {
    // Sleep so we don't get this transaction has already been processed.
    await sleep(2000);

    let listenerOne = null;
    let listenerTwo = null;

    let [eventOne, slotOne] = await new Promise((resolve, _reject) => {
      listenerOne = program.addEventListener("MyEvent", (event, slot) => {
        resolve([event, slot]);
      });
      program.rpc.initialize();
    });

    let [eventTwo, slotTwo] = await new Promise((resolve, _reject) => {
      listenerTwo = program.addEventListener("MyOtherEvent", (event, slot) => {
        resolve([event, slot]);
      });
      program.rpc.testEvent();
    });

    await program.removeEventListener(listenerOne);
    await program.removeEventListener(listenerTwo);

    assert.ok(slotOne > 0);
    assert.ok(eventOne.data.toNumber() === 5);
    assert.ok(eventOne.label === "hello");

    assert.ok(slotTwo > 0);
    assert.ok(eventTwo.data.toNumber() === 6);
    assert.ok(eventTwo.label === "bye");
  });
});

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
