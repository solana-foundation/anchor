import path from "path";
import {
  setupTest,
  runCommands,
  anchorCommand,
  diffTest,
  MOCK_BIN_DIR,
} from "@/lib";

const idlStdoutFile = path.join(__dirname, "idl-stdout");

describe("idl erase-authority", () => {
  it.skip("should succeed", () => {
    const { testDir } = setupTest({ templateName: "default-test-program" });
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);
    const outputPath = path.join(testDir, "cargo-calls");
    const validatorCallsPath = path.join(testDir, "solana-test-validator-calls");
    const rpcCallsPath = path.join(testDir, "rpc-calls");
    const walletPath = path.join(testDir, "../../../../../keypairs/aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x.json");

    const rpcMock = path.join(MOCK_BIN_DIR, "rpc");

    runCommands({
      cwd: workspaceDir,
      commands: [
        // Pre-cleanup: kill any existing process on port 8899
        `lsof -ti:8899 | xargs -r kill -9 2>/dev/null || true`,
        // Start RPC mock in background and store PID
        `MOCK_RPC_OUTPUT_PATH="${rpcCallsPath}" ${rpcMock} > /dev/null 2>&1 & RPC_PID=$!`,
        // Wait for RPC server to be ready
        `for i in {1..50}; do curl -s http://127.0.0.1:8899 > /dev/null 2>&1 && break || sleep 0.1; done`,
        // Run idl erase-authority command
        `printf "y\n" | ${anchorCommand(`idl erase-authority --program-id aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x --provider.wallet ${walletPath}`)}`,
        // Kill RPC mock after fetch finishes
        `kill $RPC_PID 2>/dev/null || true; wait $RPC_PID 2>/dev/null || true`,
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
