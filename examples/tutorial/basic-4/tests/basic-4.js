const assert = require('assert');
const anchor = require('@project-serum/anchor');

describe('basic-4', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.local());

  it('Is initialized!', async () => {
    const program = anchor.workspace.Basic4;

    // #region code
    // The data to set on the state struct.
    const data = new anchor.BN(1234);

    // Initialize the program's state struct.
    await program.state.rpc.new(data);

    // Fetch the state struct from the network.
    const state = await program.state();
    // #endregion code

    assert.ok(state.data.eq(data));
  });
});
