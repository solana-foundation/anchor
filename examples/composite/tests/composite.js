const assert = require('assert');
const anchor = require('@project-serum/anchor');

describe('composite', () => {

	const provider = anchor.Provider.local();

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  it('Is initialized!', async () => {
		const program = anchor.workspace.Composite;

		const dummyA = new anchor.web3.Account();
		const dummyB = new anchor.web3.Account();

		const tx = await program.rpc.initialize({
      accounts: {
				dummyA: dummyA.publicKey,
				dummyB: dummyB.publicKey,
				rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
				signers: [dummyA, dummyB],
      instructions: [
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: dummyA.publicKey,
          space: 8 + 8,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(
            8 + 8
          ),
          programId: program.programId,
        }),
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: dummyB.publicKey,
          space: 8 + 100,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(
            8 + 100
          ),
          programId: program.programId,
        }),
      ],
		});

			await program.rpc.compositeUpdate(new anchor.BN(1234), 'hello', {
					accounts: {
							dummyA: dummyA.publicKey,
							dummyB: dummyB.publicKey,
					},
			});

			const dummyAAccount = await program.account.dummyA(dummyA.publicKey);
			const dummyBAccount = await program.account.dummyB(dummyB.publicKey);

			assert.ok(dummyAAccount.data.eq(new anchor.BN(1234)));
			assert.ok(dummyBAccount.data === 'hello');
  });
});
