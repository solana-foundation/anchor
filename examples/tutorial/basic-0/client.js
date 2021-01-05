const anchor = require('@project-serum/anchor');

// Configure the local cluster.
anchor.setProvider(anchor.Provider.local());

async function main() {
  // #region main
  // Read the generated IDL.
  const idl = JSON.parse(require('fs').readFileSync('./target/idl/basic_0.json', 'utf8'));

  // Address of the deployed program.
  const programId = new anchor.web3.PublicKey('<YOUR-PROGRAM-ID>');

  // Generate the program client from IDL.
  const program = new anchor.Program(idl, programId);

  // Execute the RPC.
  await program.rpc.initialize();
  // #endregion main
}

console.log('Running client.');
main().then(() => console.log('Success'));
