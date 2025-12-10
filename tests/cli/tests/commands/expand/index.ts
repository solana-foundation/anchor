import fs from "fs";
import path from "path";
import { expect } from "chai";
import {
  diffTest,
  runCommands,
  setupTest,
  MOCK_BIN_DIR,
} from "@/lib";

const COMMAND_ROOT = "expand";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");
const CARGO_BIN = path.join(MOCK_BIN_DIR, "cargo");

function buildExpandCommand(...args: string[]): string {
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT} ${args.join(" ")}`;
}

describe(COMMAND_ROOT, () => {
  it("expands a single program from program directory", () => {
    const scenario = `${COMMAND_ROOT}/single-program`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildExpandCommand()} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_CARGO_PATH: CARGO_BIN,
        MOCK_ANCHOR_EXPAND_PACKAGE_NAME: "test_program",
        MOCK_ANCHOR_EXPAND_VERSION: "0.1.0",
        MOCK_ANCHOR_EXPAND_TIMESTAMP: "2024-01-01_00-00-00",
        MOCK_CARGO_EXPAND_PACKAGE_NAME: "test_program",
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Expanded test_program into file");
    expect(output).to.include(".anchor/expanded-macros/test_program/test_program-0.1.0-");
    expect(output).to.include(".rs");

    // Verify directory structure was created
    const expandedMacrosDir = path.join(workspaceDir, ".anchor", "expanded-macros");
    expect(fs.existsSync(expandedMacrosDir)).to.be.true;

    const programDir = path.join(expandedMacrosDir, "test_program");
    expect(fs.existsSync(programDir)).to.be.true;

    // Check that an expanded file was created
    const files = fs.readdirSync(programDir);
    const expandedFile = files.find(f => f.startsWith("test_program-0.1.0-") && f.endsWith(".rs"));
    expect(expandedFile).to.exist;

    diffTest(scenario);
  });

  it("expands a specific program by name", () => {
    const scenario = `${COMMAND_ROOT}/program-name`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");
    const programName = "test_program";

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildExpandCommand("--program-name", programName)} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_CARGO_PATH: CARGO_BIN,
        MOCK_ANCHOR_EXPAND_VERSION: "0.1.0",
        MOCK_ANCHOR_EXPAND_TIMESTAMP: "2024-01-01_00-00-00",
        MOCK_CARGO_EXPAND_PACKAGE_NAME: programName,
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include(`Expanded ${programName} into file`);

    // Verify directory structure
    const expandedMacrosDir = path.join(workspaceDir, ".anchor", "expanded-macros");
    expect(fs.existsSync(expandedMacrosDir)).to.be.true;

    diffTest(scenario);
  });

  it("fails when cargo-expand is not installed", () => {
    const scenario = `${COMMAND_ROOT}/cargo-expand-missing`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${buildExpandCommand()} > "${outputPath}" 2>&1`,
        ],
        env: {
          MOCK_CARGO_PATH: CARGO_BIN,
          MOCK_ANCHOR_EXPAND_PACKAGE_NAME: "test_program",
          MOCK_CARGO_EXPAND_ERROR: "error: no such subcommand: `expand`",
          MOCK_CARGO_EXPAND_EXIT_CODE: "101",
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "expand command should fail").to.be.instanceOf(Error);

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("error: no such subcommand");
  });

  it("fails when expansion encounters an error", () => {
    const scenario = `${COMMAND_ROOT}/expansion-failure`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${buildExpandCommand()} > "${outputPath}" 2>&1`,
        ],
        env: {
          MOCK_CARGO_PATH: CARGO_BIN,
          MOCK_ANCHOR_EXPAND_PACKAGE_NAME: "test_program",
          MOCK_CARGO_EXPAND_ERROR: "error: failed to expand macros",
          MOCK_CARGO_EXPAND_EXIT_CODE: "1",
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "expand command should fail").to.be.instanceOf(Error);

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("error: failed to expand macros");
  });
});

