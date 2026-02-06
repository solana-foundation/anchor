const fs = require("fs");
const idl = JSON.parse(
  fs.readFileSync("target/idl/raw_instruction.json", "utf8")
);

const rawIx = idl.instructions.find((ix) => ix.name === "raw_handler");
if (!rawIx) {
  console.error("raw_handler instruction not found");
  process.exit(1);
}

if (rawIx.args.length !== 1) {
  console.error(`Expected 1 argument, found ${rawIx.args.length}`);
  process.exit(1);
}

const arg = rawIx.args[0];
if (arg.name !== "data" || arg.type !== "bytes") {
  console.error(`Expected data: bytes, found ${arg.name}: ${arg.type}`);
  process.exit(1);
}

if (!rawIx.accounts || rawIx.accounts.length === 0) {
  console.error("raw_handler missing accounts");
  process.exit(1);
}

const initIx = idl.instructions.find((ix) => ix.name === "initialize");
if (!initIx) {
  console.error("initialize instruction not found");
  process.exit(1);
}

console.log("passed");
