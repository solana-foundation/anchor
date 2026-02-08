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
    const validatorPidFile = `/tmp/validator-${Date.now()}.pid`;
    const programName = "test-program";
    const walletPath = path.join(testDir, "../../../../keypairs/aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x.json")

    runCommands({
      cwd: workspaceDir,
      commands: [
        // Pre-cleanup: kill any existing process on port 8899
        `lsof -ti:8899 | xargs -r kill -9 2>/dev/null || true`,
        // Install trap to cleanup validator on exit (with guard check)
        `trap '[ -f "${validatorPidFile}" ] && kill $(cat "${validatorPidFile}") 2>/dev/null || true; rm -f "${validatorPidFile}"' EXIT`,
        // Run test command
        anchorCommand(`test -p ${programName} --no-idl --provider.wallet ${walletPath} --skip-deploy`),
      ],
      prependPath: [MOCK_BIN_DIR],
      env: {
        MOCK_CARGO_OUTPUT_PATH: outputPath,
        TS_NODE_OUTPUT_PATH: tsnodePath,
        MOCK_SOLANA_TEST_VALIDATOR_OUTPUT_PATH: validatorCallsPath,
        MOCK_TS_MOCHA_OUTPUT_PATH: tsMochaCallsPath,
        SOLANA_TEST_VALIDATOR_PID_FILE: validatorPidFile,
        IDL_BUILD_STDOUT_FILE: idlStdoutFile,
      },
    });

    diffTest();
  });
});
