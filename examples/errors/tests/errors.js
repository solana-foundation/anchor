const assert = require("assert");
const anchor = require('@project-serum/anchor');

describe("errors", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.local());

  const program = anchor.workspace.Errors;

  it("Emits a Hello error", async () => {
    try {
      const tx = await program.rpc.hello();
      assert.ok(false);
    } catch (err) {
      const errMsg =
        "This is an error message clients will automatically display";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 100);
    }
  });

  it("Emits a HelloNoMsg error", async () => {
    try {
      const tx = await program.rpc.helloNoMsg();
      assert.ok(false);
    } catch (err) {
      const errMsg = "HelloNoMsg";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 100 + 123);
    }
  });

  it("Emits a HelloNext error", async () => {
    try {
      const tx = await program.rpc.helloNext();
      assert.ok(false);
    } catch (err) {
      const errMsg = "HelloNext";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 100 + 124);
    }
  });
});
