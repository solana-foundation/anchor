import fs from "fs";
import path from "path";
import { expect } from "chai";
import {
  diffTest,
  runCommands,
  setupTest,
  MOCK_BIN_DIR,
} from "@/lib";

const COMMAND_ROOT = "migrate";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");

function buildMigrateCommand(): string {
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT}`;
}

describe(COMMAND_ROOT, () => {
  it("executes TypeScript migration script successfully", () => {
    const scenario = `${COMMAND_ROOT}/basic-ts`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildMigrateCommand()} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_ANCHOR_MIGRATE_USE_TS: "true",
        MOCK_ANCHOR_MIGRATE_SCRIPT_EXISTS: "true",
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Running migration deploy script");
    expect(output).to.include("Using TypeScript migration script");
    expect(output).to.include("Deploy complete.");

    diffTest(scenario);
  });

  it("executes JavaScript migration script successfully", () => {
    const scenario = `${COMMAND_ROOT}/basic-js`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildMigrateCommand()} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_ANCHOR_MIGRATE_USE_TS: "false",
        MOCK_ANCHOR_MIGRATE_SCRIPT_EXISTS: "true",
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Running migration deploy script");
    expect(output).to.include("Using JavaScript migration script");
    expect(output).to.include("Deploy complete.");

    diffTest(scenario);
  });

  it("executes custom migration script with deployment logic", () => {
    const scenario = `${COMMAND_ROOT}/custom-script`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    const customOutput = "Running migration deploy script\nDeploying programs...\nMigration completed successfully\nDeploy complete.";

    runCommands({
      cwd: workspaceDir,
      prependPath: [MOCK_BIN_DIR],
      commands: [
        `${buildMigrateCommand()} > "${outputPath}" 2>&1`,
      ],
      env: {
        MOCK_ANCHOR_MIGRATE_OUTPUT: customOutput,
        MOCK_ANCHOR_MIGRATE_SCRIPT_EXISTS: "true",
      },
    });

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Running migration deploy script");
    expect(output).to.include("Deploying programs...");
    expect(output).to.include("Migration completed successfully");
    expect(output).to.include("Deploy complete.");

    diffTest(scenario);
  });

  it("fails when migration script is missing", () => {
    const scenario = `${COMMAND_ROOT}/missing-script`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${buildMigrateCommand()} > "${outputPath}" 2>&1`,
        ],
        env: {
          MOCK_ANCHOR_MIGRATE_SCRIPT_EXISTS: "false",
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "migrate command should fail").to.be.instanceOf(Error);

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Error: Migration script not found");
  });

  it("fails when migration script encounters an error", () => {
    const scenario = `${COMMAND_ROOT}/script-error`;
    const { testDir } = setupTest(scenario);
    const workspaceDir = path.join(testDir, "test-program");
    const outputPath = path.join(testDir, "output.txt");

    let error: Error | undefined;

    try {
      runCommands({
        cwd: workspaceDir,
        prependPath: [MOCK_BIN_DIR],
        commands: [
          `${buildMigrateCommand()} > "${outputPath}" 2>&1`,
        ],
        env: {
          MOCK_ANCHOR_MIGRATE_ERROR: "Error: Migration script failed during execution",
          MOCK_ANCHOR_MIGRATE_EXIT_CODE: "1",
        },
      });
    } catch (e: any) {
      error = e;
    }

    expect(error, "migrate command should fail").to.be.instanceOf(Error);

    const output = fs.readFileSync(outputPath, "utf8");
    expect(output).to.include("Error: Migration script failed during execution");
  });
});


