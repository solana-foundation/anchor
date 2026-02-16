import path from "path";
import {
  setupTest,
  runCommands,
  anchorCommand,
  diffTest,
  MOCK_BIN_DIR,
} from "@/lib";

const idlStdoutFile = path.join(__dirname, "idl-stdout");

describe("build", () => {
  it("should succeed", () => {
    const { testDir } = setupTest({ templateName: "default-test-program" });
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);
    const outputPath = path.join(testDir, "cargo-calls");
    const programName = "test-program";
    const walletPath = path.join(testDir, "../../../../keypairs/bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ.json");

    runCommands({
      cwd: workspaceDir,
      commands: [anchorCommand(`build -p ${programName} --provider.wallet ${walletPath} --ignore-keys`)],
      prependPath: [MOCK_BIN_DIR],
      env: {
        MOCK_CARGO_OUTPUT_PATH: outputPath,
        IDL_BUILD_STDOUT_FILE: idlStdoutFile,
      },
    });

    diffTest();
  });
});
