const fs = require("fs");
const path = require("path");
const anchor = require("@anchor-lang/core");
const idl = JSON.parse(
  fs.readFileSync("target/idl/raw_instruction.json", "utf8")
);

const coder = new anchor.BorshInstructionCoder(idl);

// Test encoding a raw instruction
const rawData = Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]);
const encoded = coder.encode("raw_handler", { data: rawData });

console.log("Raw instruction encoding test:");
console.log("Input data length:", rawData.length);
console.log("Encoded length:", encoded.length);
console.log("Discriminator (first 8 bytes):", Array.from(encoded.slice(0, 8)));
console.log("Data (after discriminator):", Array.from(encoded.slice(8)));

// Verify discriminator + data structure
if (encoded.length === 8 + rawData.length && encoded.slice(8).equals(rawData)) {
  console.log("✅ Raw instruction encoding works correctly");
} else {
  console.log("❌ Raw instruction encoding failed");
  console.log("Expected length:", 8 + rawData.length, "Got:", encoded.length);
  process.exit(1);
}

// Test decoding
const decoded = coder.decode(encoded);
if (
  decoded &&
  decoded.name === "raw_handler" &&
  Buffer.from(decoded.data.data).equals(rawData)
) {
  console.log("✅ Raw instruction decoding works correctly");
} else {
  console.log("❌ Raw instruction decoding failed");
  console.log("Decoded:", decoded);
  process.exit(1);
}

console.log("\n✅ All encoding/decoding tests passed!");
