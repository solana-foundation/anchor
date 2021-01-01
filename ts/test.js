const anchor = require('.');

console.log(anchor)
function test() {
		const fs = require('fs');
		const idl = JSON.parse(fs.readFileSync('../examples/basic/idl.json', 'utf8'));
		const program = new anchor.Program(idl);
}

test();
