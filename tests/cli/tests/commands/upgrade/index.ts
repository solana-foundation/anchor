import fs from "fs";
import path from "path";
import { expect } from "chai";
import {
  anchorCommand,
  diffTest,
  replaceInFile,
  runCommands,
  setupTest,
  MOCK_BIN_DIR,
} from "@/lib";

const COMMAND_ROOT = "upgrade";

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
  upgradeOutput,
  workspaceDir,
  testDir,
}: {
  upgradeOutput: string;
  workspaceDir: string;
  testDir: string;
}): void {
  const homeDir = process.env.HOME;
  const walletPath = homeDir
    ? path.join(homeDir, ".config", "solana", "id.json")
    : undefined;

  normalizeFile(upgradeOutput, {
    [workspaceDir]: "<WORKSPACE_DIR>",
    [testDir]: "<TEST_DIR>",
    [walletPath ?? ""]: "<WALLET>",
  });

  // Normalize signature to a fixed pattern
  if (fs.existsSync(upgradeOutput)) {
    let content = fs.readFileSync(upgradeOutput, "utf8");
    content = content.replace(/Signature: [A-Za-z0-9]{88}/g, "Signature: <SIGNATURE>");
    fs.writeFileSync(upgradeOutput, content);
  }
}

describe(COMMAND_ROOT, () => {
  it("upgrades a program with basic configuration", () => {
    const scenario = `${COMMAND_ROOT}/basic`;
    const { testDir } = setupTest(scenario);
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);
    const programName = "test-program";
    const programId = "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x";

    ensureBinary(workspaceDir, programName);

    const upgradeOutput = path.join(testDir, "upgrade-output.txt");
    const binaryPath = path.join(workspaceDir, "target", "deploy", "test_program.so");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${anchorCommand(
          `upgrade ${binaryPath} --program-id ${programId} -- --with-compute-unit-price 0`,
        )} > "${upgradeOutput}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_UPGRADE_PROGRAM_ID: programId,
        MOCK_SOLANA_SIGNATURE: "5" + "X".repeat(87),
      },
    });

    normalizeOutputs({ upgradeOutput, workspaceDir, testDir });

    const output = fs.readFileSync(upgradeOutput, "utf8");
    expect(output).to.include(`Program Id: ${programId}`);
    expect(output).to.include("Signature:");

    diffTest(scenario);
  });

  it("upgrades with custom program ID", () => {
    const scenario = `${COMMAND_ROOT}/custom-program-id`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const programName = "test-program";
    const customProgramId = "bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ";

    ensureBinary(workspaceDir, programName);

    const upgradeOutput = path.join(testDir, "upgrade-output.txt");
    const binaryPath = path.join(workspaceDir, "target", "deploy", "test_program.so");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${anchorCommand(
          `upgrade ${binaryPath} --program-id ${customProgramId} -- --with-compute-unit-price 0`,
        )} > "${upgradeOutput}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_UPGRADE_PROGRAM_ID: customProgramId,
        MOCK_SOLANA_SIGNATURE: "5" + "Y".repeat(87),
      },
    });

    normalizeOutputs({ upgradeOutput, workspaceDir, testDir });

    const output = fs.readFileSync(upgradeOutput, "utf8");
    expect(output).to.include(`Program Id: ${customProgramId}`);
    expect(output).to.include("Signature:");

    diffTest(scenario);
  });

  it("forwards additional solana arguments", () => {
    const scenario = `${COMMAND_ROOT}/solana-args`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const programName = "test-program";
    const programId = "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x";

    ensureBinary(workspaceDir, programName);

    const upgradeOutput = path.join(testDir, "upgrade-output.txt");
    const binaryPath = path.join(workspaceDir, "target", "deploy", "test_program.so");

    const extraArg = "--with-compute-unit-price";
    const extraValue = "5000";

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${anchorCommand(
          `upgrade ${binaryPath} --program-id ${programId} -- ${extraArg} ${extraValue}`,
        )} > "${upgradeOutput}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_UPGRADE_PROGRAM_ID: programId,
      },
    });

    normalizeOutputs({ upgradeOutput, workspaceDir, testDir });

    const output = fs.readFileSync(upgradeOutput, "utf8");
    expect(output).to.include(`Program Id: ${programId}`);

    diffTest(scenario);
  });

  it("handles max retries on failure", () => {
    const scenario = `${COMMAND_ROOT}/max-retries`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const programName = "test-program";
    const programId = "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x";

    ensureBinary(workspaceDir, programName);

    const upgradeOutput = path.join(testDir, "upgrade-output.txt");
    const binaryPath = path.join(workspaceDir, "target", "deploy", "test_program.so");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
      commands: [
        `${anchorCommand(
          `upgrade ${binaryPath} --program-id ${programId} --max-retries 2 -- --with-compute-unit-price 0`,
        )} > "${upgradeOutput}" 2>&1`,
      ],
        env: {
          MOCK_SOLANA_UPGRADE_ERROR: "Error: Transaction simulation failed",
          MOCK_SOLANA_UPGRADE_EXIT_CODE: "1",
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "upgrade command should fail").to.be.instanceOf(Error);
  });

  it("fails when binary file is missing", () => {
    const scenario = `${COMMAND_ROOT}/missing-binary`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const programId = "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x";

    // Intentionally NOT creating the binary file
    const binaryPath = path.join(workspaceDir, "target", "deploy", "test_program.so");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${anchorCommand(
            `upgrade ${binaryPath} --program-id ${programId} -- --with-compute-unit-price 0`,
          )} 2>&1`,
        ],
        env: {
          MOCK_SOLANA_UPGRADE_PROGRAM_ID: programId,
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "upgrade should fail when binary is missing").to.be.instanceOf(Error);
  });

  it("fails with invalid program ID", () => {
    const scenario = `${COMMAND_ROOT}/invalid-program-id`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const programName = "test-program";
    const invalidProgramId = "invalid-id";

    ensureBinary(workspaceDir, programName);

    const binaryPath = path.join(workspaceDir, "target", "deploy", "test_program.so");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${anchorCommand(
            `upgrade ${binaryPath} --program-id ${invalidProgramId} -- --with-compute-unit-price 0`,
          )} 2>&1`,
        ],
        env: {
          MOCK_SOLANA_UPGRADE_ERROR: "Error: Invalid program ID format",
          MOCK_SOLANA_UPGRADE_EXIT_CODE: "1",
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "upgrade should fail with invalid program ID").to.be.instanceOf(Error);
  });
});

