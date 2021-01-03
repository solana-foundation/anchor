// TODO: replace path once the package is published.
//
// Before running this script, make sure to run `yarn && yarn build` inside
// the `ts` directory.
const anchor = require('../../../../ts');
const fs = require('fs');

// Configure the local cluster.
anchor.setProvider(anchor.Provider.local());

// #region main
async function main() {
  // Read the generated IDL.
  const idl = JSON.parse(fs.readFileSync('../idl.json', 'utf8'));

  // Address of the deployed program.
  const programId = new anchor.web3.PublicKey('<YOUR-PROGRAM-ID>');

  // Generate the program client from IDL.
  const program = new anchor.Program(idl, programId);

  // Execute the RPC.
  await program.rpc.initialize();
}
// #endregion main

main();
