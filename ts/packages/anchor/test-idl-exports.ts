// test-idl-exports.ts
// run npx tsc --noEmit --skipLibCheck test-idl-exports.ts in root to verify type imports

// BEFORE FIX - These imports will FAIL
import type { 
  IdlErrorCode,
  IdlEvent, 
  IdlField,
  IdlInstruction,
  IdlAccount,
  IdlType
} from "@coral-xyz/anchor";

//  This should work both before and after
import type { Idl } from "@coral-xyz/anchor";

// Simple usage test
const errorCode: IdlErrorCode = {
  name: "CustomError",
  code: 6000,
  msg: "Something went wrong"
};

const event: IdlEvent = {
  name: "MyEvent",
  discriminator: [1, 2, 3, 4, 5, 6, 7, 8]
};

const field: IdlField = {
  name: "amount",
  type: "u64"
};

console.log(" All IDL types imported successfully!");
console.log({ errorCode, event, field });