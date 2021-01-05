const anchor = require('@project-serum/anchor');

describe('basic-2', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.local());

  it('Applies constraints and access control', async () => {
    const program = anchor.workspace.Basic2;
  });
});
