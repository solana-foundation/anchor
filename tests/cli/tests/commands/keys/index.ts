import { runKeysListTests } from "./list";
import { runKeysSyncTests } from "./sync";

describe("keys", () => {
  runKeysListTests();
  runKeysSyncTests();
});

