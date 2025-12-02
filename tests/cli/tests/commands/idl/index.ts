import { runIdlBuildTests } from "./build";
import { runIdlInitTests } from "./init";

describe("idl", () => {
  runIdlBuildTests();
  runIdlInitTests();
});


