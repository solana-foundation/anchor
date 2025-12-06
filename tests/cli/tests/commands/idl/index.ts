import { runIdlBuildTests } from "./build";
import { runIdlFetchTests } from "./fetch";
import { runIdlInitTests } from "./init";
import { runIdlAuthorityTests } from "./authority";

describe("idl", () => {
  runIdlBuildTests();
  runIdlFetchTests();
  runIdlInitTests();
  runIdlAuthorityTests();
});


