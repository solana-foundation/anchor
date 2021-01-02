const assert = require('assert');
const anchor = require('.');

// Global workspace settings.
const WORKSPACE = {
    idl: JSON.parse(require('fs').readFileSync('../examples/basic/idl.json', 'utf8')),
    programId: new anchor.web3.PublicKey('CrQZpSbUnkXxwf1FnepmefoZ7VsbYE6HXmG1TjChH6y'),
    provider: anchor.Provider.local(),
};

async function test() {
    console.log('Starting test.');

    // Configure the local cluster.
    anchor.setProvider(WORKSPACE.provider);

    // Generate the program from IDL.
    const program = new anchor.Program(WORKSPACE.idl, WORKSPACE.programId);

    // New account to create.
    const root = new anchor.web3.Account();

    // Execute the RPC (instruction) against the cluster, passing in the arguments
    // exactly as defined by the Solana program.
    //
    // The last parameter defines context for the transaction. Consisting of
    //
    // 1) Any additional instructions one wishes to execute *before* executing
    //    the program.
    // 2) Any signers (in addition to the provider).
    // 3) Accounts for the program's instruction. Ordering does *not* matter,
    //    only that they names are as specified in the IDL.
    await program.rpc.createRoot(WORKSPACE.provider.wallet.publicKey, new anchor.BN(1234), {
        accounts: {
            root: root.publicKey,
        },
        signers: [root],
        instructions: [
            anchor.web3.SystemProgram.createAccount({
                fromPubkey: WORKSPACE.provider.wallet.publicKey,
                newAccountPubkey: root.publicKey,
                space: 41,
                lamports: await WORKSPACE.provider.connection.getMinimumBalanceForRentExemption(41),
                programId: WORKSPACE.programId,
            }),
        ],
    });

    // Read the newly created account data.
    let account = await program.account.root(root.publicKey);
    assert.ok(account.initialized);
    assert.ok(account.data.eq(new anchor.BN(1234)));
    assert.ok(account.authority.equals(WORKSPACE.provider.wallet.publicKey));

    // Execute another RPC to update the data.
    await program.rpc.updateRoot(new anchor.BN(999), {
        accounts: {
            root: root.publicKey,
            authority: WORKSPACE.provider.wallet.publicKey,
        },
    });

    // Check the update actually persisted.
    account = await program.account.root(root.publicKey);
    assert.ok(account.data.eq(new anchor.BN(999)));

    // Create and initialize a leaf account.
    const leaf = new anchor.web3.Account();
    let customType = { myData: new anchor.BN(4), key: WORKSPACE.programId };
    await program.rpc.createLeaf(new anchor.BN(2), customType, {
        accounts: {
            root: root.publicKey,
            leaf: leaf.publicKey,
        },
        signers: [leaf],
        instructions: [
            anchor.web3.SystemProgram.createAccount({
                fromPubkey: WORKSPACE.provider.wallet.publicKey,
                newAccountPubkey: leaf.publicKey,
                space: 100,
                lamports: await WORKSPACE.provider.connection.getMinimumBalanceForRentExemption(100),
                programId: WORKSPACE.programId,
            }),
        ],
    });

    // Check the account was initialized.
    account = await program.account.leaf(leaf.publicKey);
    assert.ok(account.initialized);
    assert.ok(account.root.equals(root.publicKey));
    assert.ok(account.data.eq(new anchor.BN(2)));
    assert.ok(account.custom.myData.eq(new anchor.BN(4)));
    assert.ok(account.custom.key.equals(WORKSPACE.programId));

    // Update the account.
    await program.rpc.updateLeaf(new anchor.BN(5), null, {
        accounts: {
            authority: WORKSPACE.provider.wallet.publicKey,
            root: root.publicKey,
            leaf: leaf.publicKey,
        },
    });

    // Check it was updated.
    account = await program.account.leaf(leaf.publicKey);
    assert.ok(account.data.eq(new anchor.BN(5)));

    // Now update with the option.
    customType = { myData: new anchor.BN(7), key: WORKSPACE.programId };
    await program.rpc.updateLeaf(new anchor.BN(6), customType, {
        accounts: {
            authority: WORKSPACE.provider.wallet.publicKey,
            root: root.publicKey,
            leaf: leaf.publicKey,
        },
    });

    // Check it was updated.
    account = await program.account.leaf(leaf.publicKey);
    assert.ok(account.data.eq(new anchor.BN(6)));
    assert.ok(account.custom.myData.eq(new anchor.BN(7)));
    assert.ok(account.custom.key.equals(WORKSPACE.programId));

    console.log('Test complete.');
}

test();
