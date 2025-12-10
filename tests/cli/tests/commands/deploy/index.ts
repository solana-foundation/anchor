import fs from "fs";
import path from "path";
import { expect } from "chai";
import {
  anchorCommand,
  diffTest,
  patchWorkspace,
  replaceInFile,
  runCommands,
  setupTest,
  MOCK_BIN_DIR,
} from "@/lib";

const COMMAND_ROOT = "deploy";

function ensureBinary(workspaceDir: string, programName: string): void {
  const programRustName = programName.replace(/-/g, "_");
  const deployDir = path.join(workspaceDir, "target", "deploy");
  fs.mkdirSync(deployDir, { recursive: true });
  const binaryPath = path.join(deployDir, `${programRustName}.so`);
  if (!fs.existsSync(binaryPath)) {
    fs.writeFileSync(binaryPath, "");
  }
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function normalizeFile(
  filePath: string,
  replacements: Record<string, string | undefined>,
): void {
  if (!filePath) return;
  for (const [from, to] of Object.entries(replacements)) {
    if (!from || to === undefined) continue;
    replaceInFile({
      file: filePath,
      find: new RegExp(escapeRegExp(from), "g"),
      replace: to,
    });
  }
}

function normalizeOutputs({
  deployOutput,
  solanaLog,
  workspaceDir,
  testDir,
}: {
  deployOutput: string;
  solanaLog: string;
  workspaceDir: string;
  testDir: string;
}): void {
  const homeDir = process.env.HOME;
  const walletPath = homeDir
    ? path.join(homeDir, ".config", "solana", "id.json")
    : undefined;

  normalizeFile(deployOutput, {
    [workspaceDir]: "<WORKSPACE_DIR>",
    [testDir]: "<TEST_DIR>",
    [walletPath ?? ""]: "<WALLET>",
  });

  normalizeFile(solanaLog, {
    [walletPath ?? ""]: "<WALLET>",
  });
}

describe(COMMAND_ROOT, () => {
  it("deploys all programs with default configuration", () => {
    const scenario = `${COMMAND_ROOT}/basic`;
    const { testDir } = setupTest(scenario);
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);
    const programName = "test-program";

    ensureBinary(workspaceDir, programName);

    const deployOutput = path.join(testDir, "deploy-output.txt");
    const solanaLog = path.join(testDir, "solana-log.jsonl");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${anchorCommand(
          "deploy --no-idl -- --with-compute-unit-price 0",
        )} > "${deployOutput}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_LOG_PATH: solanaLog,
      },
    });

    normalizeOutputs({ deployOutput, solanaLog, workspaceDir, testDir });

    patchWorkspace({ workspaceDir });

    diffTest(scenario);
  });

  it("deploys only the selected program when program-name flag is provided", () => {
    const scenario = `${COMMAND_ROOT}/program-name`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const programName = "another-program";

    ensureBinary(workspaceDir, programName);

    const deployOutput = path.join(testDir, "deploy-output.txt");
    const solanaLog = path.join(testDir, "solana-log.jsonl");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${anchorCommand(
          `deploy --no-idl --program-name ${programName} -- --with-compute-unit-price 0`,
        )} > "${deployOutput}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_LOG_PATH: solanaLog,
      },
    });

    normalizeOutputs({ deployOutput, solanaLog, workspaceDir, testDir });

    patchWorkspace({ workspaceDir });

    diffTest(scenario);
  });

  it("deploys using a custom program keypair", () => {
    const scenario = `${COMMAND_ROOT}/program-keypair`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const programName = "test-program";

    ensureBinary(workspaceDir, programName);

    const deployOutput = path.join(testDir, "deploy-output.txt");
    const solanaLog = path.join(testDir, "solana-log.jsonl");
    const keypairPath = path.join(workspaceDir, "keypairs", "custom.json");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${anchorCommand(
          `deploy --no-idl --program-name ${programName} --program-keypair ${keypairPath} -- --with-compute-unit-price 0`,
        )} > "${deployOutput}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_LOG_PATH: solanaLog,
      },
    });

    normalizeOutputs({ deployOutput, solanaLog, workspaceDir, testDir });
    normalizeFile(deployOutput, {
      [keypairPath]: "<CUSTOM_KEYPAIR>",
    });

    patchWorkspace({ workspaceDir });

    diffTest(scenario);
  });

  it("forwards additional solana arguments", () => {
    const scenario = `${COMMAND_ROOT}/solana-args`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const programName = "test-program";

    ensureBinary(workspaceDir, programName);

    const deployOutput = path.join(testDir, "deploy-output.txt");
    const solanaLog = path.join(testDir, "solana-log.jsonl");

    const extraArg = "--commitment";
    const extraValue = "confirmed";

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${anchorCommand(
          `deploy --no-idl -- --with-compute-unit-price 0 ${extraArg} ${extraValue}`,
        )} > "${deployOutput}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_LOG_PATH: solanaLog,
      },
    });

    normalizeOutputs({ deployOutput, solanaLog, workspaceDir, testDir });

    patchWorkspace({ workspaceDir });

    diffTest(scenario);
  });

  it("fails when the solana CLI returns a non-zero status", () => {
    const scenario = `${COMMAND_ROOT}/failure`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const programName = "test-program";

    ensureBinary(workspaceDir, programName);

    const solanaLog = path.join(testDir, "solana-log.jsonl");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${anchorCommand("deploy --no-idl -- --with-compute-unit-price 0")}`,
        ],
        env: {
          MOCK_SOLANA_LOG_PATH: solanaLog,
          MOCK_SOLANA_FORCE_EXIT_CODE: "1",
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "deploy command should fail").to.be.instanceOf(Error);
    expect(error?.message ?? "", "deploy error message").to.match(
      /There was a problem deploying/,
    );

    const homeDir = process.env.HOME;
    const walletPath = homeDir
      ? path.join(homeDir, ".config", "solana", "id.json")
      : undefined;
    normalizeFile(solanaLog, {
      [walletPath ?? ""]: "<WALLET>",
    });
    patchWorkspace({ workspaceDir });

    diffTest(scenario);
  });
});

