const web3 = require('@solana/web3.js');
const anchor = require('.');
anchor.setProvider(anchor.Provider.local());

const idl = JSON.parse(require('fs').readFileSync('../examples/basic/idl.json', 'utf8'));
const pid = new web3.PublicKey('9gzNv4hUB1F3jQQNNcZxxjn1bCjgaTCrucDjFh2i8vc6');

async function test() {
    const program = new anchor.Program(idl, pid);
    const sig = await program.rpc.createRoot(
      new PublicKey(''),
      1234,
    );
}

test();
