import { runIdlBuildTests } from "./build";
import { runIdlFetchTests } from "./fetch";
import { runIdlInitTests } from "./init";
import { runIdlAuthorityTests } from "./authority";
import { runIdlEraseAuthorityTests } from "./erase-authority";
import { runIdlUpgradeTests } from "./upgrade";

describe("idl", () => {
  runIdlBuildTests();
  runIdlFetchTests();
  runIdlInitTests();
  runIdlAuthorityTests();
  runIdlEraseAuthorityTests();
  runIdlUpgradeTests();
});


