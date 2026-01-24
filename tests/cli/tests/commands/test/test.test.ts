import path from "path";
import {
  setupTest,
  runCommands,
  anchorCommand,
  diffTest,
  MOCK_BIN_DIR,
} from "@/lib";

const idlStdoutFile = path.join(__dirname, "idl-stdout");

describe("test", () => {
  it("should succeed", () => {
    const { testDir } = setupTest({ templateName: "default-test-program" });
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);
    const outputPath = path.join(testDir, "cargo-calls");
    const tsnodePath = path.join(testDir, "ts-node");
    const validatorCallsPath = path.join(testDir, "solana-test-validator-calls");
    const tsMochaCallsPath = path.join(testDir, "ts-mocha-calls");
    const programName = "test-program";
    const walletPath = path.join(testDir, "../../../../keypairs/aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x.json")

    runCommands({
      cwd: workspaceDir,
      commands: [anchorCommand(`test -p ${programName} --no-idl --provider.wallet ${walletPath} --skip-deploy`)],
      prependPath: [MOCK_BIN_DIR],
      env: {
        MOCK_CARGO_OUTPUT_PATH: outputPath,
        TS_NODE_OUTPUT_PATH: tsnodePath,
        MOCK_SOLANA_TEST_VALIDATOR_OUTPUT_PATH: validatorCallsPath,
        MOCK_TS_MOCHA_OUTPUT_PATH: tsMochaCallsPath,
        IDL_BUILD_STDOUT_FILE: idlStdoutFile,
      },
    });

    diffTest();
  });
});
