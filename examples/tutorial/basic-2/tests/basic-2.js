const assert = require('assert');
const anchor = require('@project-serum/anchor');

describe('basic-2', () => {
  const provider = anchor.Provider.local()

  // Configure the client to use the local cluster.
  anchor.setProvider(provider)

  // Counter for the tests.
  const counter = anchor.web3.Keypair.generate()

  // Program for the tests.
  const program = anchor.workspace.Basic2

  it('Creates a counter', async () => {
    await program.rpc.create(provider.wallet.publicKey, {
      accounts: {
        counter: counter.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [counter],
      instructions: [await program.account.counter.createInstruction(counter)],
    })

    let counterAccount = await program.account.counter.fetch(counter.publicKey)

    assert.ok(counterAccount.authority.equals(provider.wallet.publicKey))
    assert.ok(counterAccount.count.toNumber() === 0)
  })

  it('Updates a counter', async () => {
    await program.rpc.increment({
      accounts: {
        counter: counter.publicKey,
        authority: provider.wallet.publicKey,
      },
    })

    const counterAccount = await program.account.counter.fetch(counter.publicKey)

    assert.ok(counterAccount.authority.equals(provider.wallet.publicKey))
    assert.ok(counterAccount.count.toNumber() == 1)
  })
})
