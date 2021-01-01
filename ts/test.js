const anchor = require('.');

function test() {
		const fs = require('fs');
		const idl = JSON.parse(fs.readFileSync('../examples/basic/idl.json', 'utf8'));
		const pid = '9gzNv4hUB1F3jQQNNcZxxjn1bCjgaTCrucDjFh2i8vc6';
		const program = new anchor.Program(idl, pid);

		/*
		const ctx = {
				authority:
		};
		program.rpc.updateLeaf();
		*/

		console.log('RPCS', program.rpc);
		console.log('IXS', program.instruction);
		console.log('Accounts', program.account);
}

test();
