const anchor = require('@project-serum/anchor');

describe('sysvars', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.local());

  it('Is initialized!', async () => {
    const program = anchor.workspace.Sysvars;
    const tx = await program.rpc.sysvars({
      accounts: {
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        stakeHistory: anchor.web3.SYSVAR_STAKE_HISTORY_PUBKEY,
      },
    });
    console.log("Your transaction signature", tx);
  });
});
