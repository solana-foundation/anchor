import * as anchor from '@anchor-lang/core';

// Deploy script defined by the user.
const userScript = require("/workspaces/devpool/anchor/tests/cli/tests/commands/migrate/output/test-program/migrations/deploy.ts");

async function main() {
    const connection = new anchor.web3.Connection(
      "http://127.0.0.1:8899",
      anchor.AnchorProvider.defaultOptions().commitment
    );
    const wallet = anchor.Wallet.local();
    const provider = new anchor.AnchorProvider(connection, wallet);

    // Run the user's deploy script.
    userScript(provider);
}
main();
