import { runIdlBuildTests } from "./build";
import { runIdlFetchTests } from "./fetch";
import { runIdlInitTests } from "./init";

describe("idl", () => {
  runIdlBuildTests();
  runIdlFetchTests();
  runIdlInitTests();
});


