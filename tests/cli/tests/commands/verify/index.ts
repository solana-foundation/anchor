import fs from "fs";
import path from "path";
import { expect } from "chai";
import {
  diffTest,
  runCommands,
  setupTest,
  MOCK_BIN_DIR,
} from "@/lib";

const COMMAND_ROOT = "verify";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");
const SOLANA_VERIFY_BIN = path.join(MOCK_BIN_DIR, "solana-verify");
const TEST_PROGRAM_ID = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

function buildVerifyCommand(programId: string, ...args: string[]): string {
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT} ${programId} ${args.join(" ")}`;
}

describe(COMMAND_ROOT, () => {
  it("verifies program with current directory", () => {
    const scenario = `${COMMAND_ROOT}/current-dir`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildVerifyCommand(TEST_PROGRAM_ID, "--current-dir")} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_VERIFY_PATH: SOLANA_VERIFY_BIN,
        MOCK_SOLANA_VERIFY_PROGRAM_ID: TEST_PROGRAM_ID,
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include(`Verifying program ${TEST_PROGRAM_ID}`);
    expect(output).to.include("Verification successful!");
    expect(output).to.include("On-chain bytecode matches the local source code.");

    diffTest(scenario);
  });

  it("verifies program with repository URL", () => {
    const scenario = `${COMMAND_ROOT}/repo-url`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");
    const repoUrl = "https://github.com/coral-xyz/anchor";

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildVerifyCommand(TEST_PROGRAM_ID, "--repo-url", repoUrl)} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_VERIFY_PATH: SOLANA_VERIFY_BIN,
        MOCK_SOLANA_VERIFY_PROGRAM_ID: TEST_PROGRAM_ID,
        MOCK_SOLANA_VERIFY_REPO_URL: repoUrl,
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include(`Verifying program ${TEST_PROGRAM_ID}`);
    expect(output).to.include(`from repository: ${repoUrl}`);
    expect(output).to.include("Verification successful!");

    diffTest(scenario);
  });

  it("verifies program with commit hash", () => {
    const scenario = `${COMMAND_ROOT}/commit-hash`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");
    const repoUrl = "https://github.com/coral-xyz/anchor";
    const commitHash = "abc123def456";

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildVerifyCommand(TEST_PROGRAM_ID, "--repo-url", repoUrl, "--commit-hash", commitHash)} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_VERIFY_PATH: SOLANA_VERIFY_BIN,
        MOCK_SOLANA_VERIFY_PROGRAM_ID: TEST_PROGRAM_ID,
        MOCK_SOLANA_VERIFY_REPO_URL: repoUrl,
        MOCK_SOLANA_VERIFY_COMMIT_HASH: commitHash,
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include(`Verifying program ${TEST_PROGRAM_ID}`);
    expect(output).to.include(`from repository: ${repoUrl}`);
    expect(output).to.include(`Using commit: ${commitHash}`);
    expect(output).to.include("Verification successful!");

    diffTest(scenario);
  });

  it("verifies program with custom program name", () => {
    const scenario = `${COMMAND_ROOT}/program-name`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");
    const programName = "my_custom_program";

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildVerifyCommand(TEST_PROGRAM_ID, "--current-dir", "--program-name", programName)} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_SOLANA_VERIFY_PATH: SOLANA_VERIFY_BIN,
        MOCK_SOLANA_VERIFY_PROGRAM_ID: TEST_PROGRAM_ID,
        MOCK_SOLANA_VERIFY_PROGRAM_NAME: programName,
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include(`Verifying program ${TEST_PROGRAM_ID}`);
    expect(output).to.include(`Library name: ${programName}`);
    expect(output).to.include("Verification successful!");

    diffTest(scenario);
  });

  it("fails when neither --current-dir nor --repo-url is provided", () => {
    const scenario = `${COMMAND_ROOT}/missing-args`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${buildVerifyCommand(TEST_PROGRAM_ID)} > "${outputPath}" 2>&1`,
        ],
        env: {
          MOCK_SOLANA_VERIFY_PATH: SOLANA_VERIFY_BIN,
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "verify command should fail").to.be.instanceOf(Error);

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Error: You must provide either --repo-url or --current-dir");
  });

  it("fails when verification process fails", () => {
    const scenario = `${COMMAND_ROOT}/verification-failure`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${buildVerifyCommand(TEST_PROGRAM_ID, "--current-dir")} > "${outputPath}" 2>&1`,
        ],
        env: {
          MOCK_SOLANA_VERIFY_PATH: SOLANA_VERIFY_BIN,
          MOCK_SOLANA_VERIFY_PROGRAM_ID: TEST_PROGRAM_ID,
          MOCK_SOLANA_VERIFY_ERROR: "Error: Verification failed - bytecode mismatch",
          MOCK_SOLANA_VERIFY_EXIT_CODE: "1",
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "verify command should fail").to.be.instanceOf(Error);

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include(`Verifying program ${TEST_PROGRAM_ID}`);
    expect(output).to.include("Error: Verification failed - bytecode mismatch");
  });
});

