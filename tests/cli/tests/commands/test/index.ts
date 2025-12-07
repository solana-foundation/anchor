import fs from "fs";
import path from "path";
import { expect } from "chai";
import {
  diffTest,
  runCommands,
  setupTest,
  MOCK_BIN_DIR,
} from "@/lib";

const COMMAND_ROOT = "test";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");

function buildTestCommand(extraArgs = ""): string {
  const suffix = extraArgs ? ` ${extraArgs}` : "";
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT}${suffix}`;
}

describe(COMMAND_ROOT, () => {
  it("runs tests with default configuration", () => {
    const scenario = `${COMMAND_ROOT}/basic`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildTestCommand()} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_ANCHOR_TEST_SKIP_DEPLOY: "false",
        MOCK_ANCHOR_TEST_SKIP_BUILD: "false",
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Building programs...");
    expect(output).to.include("Deploying programs...");
    expect(output).to.include("Starting local test validator...");
    expect(output).to.include("1 passing");

    diffTest(scenario);
  });

  it("skips deployment when --skip-deploy flag is used", () => {
    const scenario = `${COMMAND_ROOT}/skip-deploy`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildTestCommand("--skip-deploy")} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_ANCHOR_TEST_SKIP_DEPLOY: "true",
        MOCK_ANCHOR_TEST_SKIP_BUILD: "false",
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Building programs...");
    expect(output).to.not.include("Deploying programs...");
    expect(output).to.include("1 passing");

    diffTest(scenario);
  });

  it("skips build when --skip-build flag is used", () => {
    const scenario = `${COMMAND_ROOT}/skip-build`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildTestCommand("--skip-build")} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_ANCHOR_TEST_SKIP_DEPLOY: "false",
        MOCK_ANCHOR_TEST_SKIP_BUILD: "true",
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.not.include("Building programs...");
    expect(output).to.include("Deploying programs...");
    expect(output).to.include("1 passing");

    diffTest(scenario);
  });

  it("skips local validator when --skip-local-validator flag is used", () => {
    const scenario = `${COMMAND_ROOT}/skip-validator`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildTestCommand("--skip-local-validator")} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_ANCHOR_TEST_SKIP_DEPLOY: "false",
        MOCK_ANCHOR_TEST_SKIP_BUILD: "false",
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Building programs...");
    expect(output).to.include("Deploying programs...");
    expect(output).to.not.include("Starting local test validator...");
    expect(output).to.include("1 passing");

    diffTest(scenario);
  });

  it("tests only specified program with --program-name flag", () => {
    const scenario = `${COMMAND_ROOT}/program-name`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");
    const programName = "test_program";

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildTestCommand(`--program-name ${programName}`)} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_ANCHOR_TEST_PROGRAM_NAME: programName,
        MOCK_ANCHOR_TEST_SKIP_DEPLOY: "false",
        MOCK_ANCHOR_TEST_SKIP_BUILD: "false",
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Building programs...");
    expect(output).to.include("1 passing");

    diffTest(scenario);
  });

  it("fails when test suite encounters an error", () => {
    const scenario = `${COMMAND_ROOT}/test-failure`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${buildTestCommand()} > "${outputPath}" 2>&1`,
        ],
        env: {
          MOCK_ANCHOR_TEST_ERROR: "Error: Test suite failed",
          MOCK_ANCHOR_TEST_EXIT_CODE: "1",
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "test command should fail").to.be.instanceOf(Error);

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Error: Test suite failed");
  });
});

