import path from "path";
import {
  setupTest,
  runCommands,
  anchorCommand,
  diffTest,
  MOCK_BIN_DIR,
} from "@/lib";

const idlStdoutFile = path.join(__dirname, "idl-stdout");

describe("idl build", () => {
  it("should succeed", () => {
    const { testDir } = setupTest({ templateName: "default-test-program" });
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);
    const outputPath = path.join(testDir, "cargo-calls");
    const validatorCallsPath = path.join(testDir, "solana-test-validator-calls");
    const rpcCallsPath = path.join(testDir, "rpc-calls");
    const walletPath = path.join(testDir, "../../../../keypairs/aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x.json")

    runCommands({
      cwd: workspaceDir,
      commands: [
        anchorCommand(`idl build -p test-program --provider.wallet ${walletPath}`),
      ],
      prependPath: [MOCK_BIN_DIR],
      env: {
        MOCK_CARGO_OUTPUT_PATH: outputPath,
        MOCK_SOLANA_TEST_VALIDATOR_OUTPUT_PATH: validatorCallsPath,
        MOCK_RPC_OUTPUT_PATH: rpcCallsPath,
        IDL_BUILD_STDOUT_FILE: idlStdoutFile,
      },
    });

    diffTest();
  });
});
