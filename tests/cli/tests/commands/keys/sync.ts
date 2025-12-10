import fs from "fs";
import path from "path";
import { expect } from "chai";
import { diffTest, runCommands, setupTest, MOCK_BIN_DIR } from "@/lib";

const COMMAND_ROOT = "keys";
const SUBCOMMAND = "sync";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");

function buildSyncCommand(extraArgs = ""): string {
  const suffix = extraArgs ? ` ${extraArgs}` : "";
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT} ${SUBCOMMAND}${suffix}`;
}

const KEYS_SYNC_ROOT = `${COMMAND_ROOT}/${SUBCOMMAND}`;

export function runKeysSyncTests() {
  describe(`${COMMAND_ROOT} ${SUBCOMMAND}`, () => {
    it("reports all programs are synced when they match", () => {
      const scenario = path.posix.join(KEYS_SYNC_ROOT, "basic");
      const { testDir } = setupTest(scenario);
      const outputPath = path.join(testDir, "output.txt");

      runCommands({
        cwd: path.join(testDir, "test-program"),
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_KEYS_SYNC_CLUSTER: "localnet",
          MOCK_KEYS_SYNC_CHANGES_MADE: "false",
        },
        commands: [`${buildSyncCommand()} > "${outputPath}" 2>&1`],
      });

      const output = fs.readFileSync(outputPath, "utf8");
      expect(output).to.include("Syncing program ids for the configured cluster (localnet)");
      expect(output).to.include("All program id declarations are synced.");
      expect(output).to.not.include("Please rebuild");

      diffTest(scenario);
    });

    it("syncs program ids when they are out of sync", () => {
      const scenario = path.posix.join(KEYS_SYNC_ROOT, "out-of-sync");
      const { testDir } = setupTest(scenario);
      const outputPath = path.join(testDir, "output.txt");

      runCommands({
        cwd: path.join(testDir, "test-program"),
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_KEYS_SYNC_CLUSTER: "localnet",
          MOCK_KEYS_SYNC_CHANGES_MADE: "true",
          MOCK_KEYS_SYNC_SOURCE_CHANGES: JSON.stringify([
            {
              file: "programs/test-program/src/lib.rs",
              newId: "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
            },
          ]),
          MOCK_KEYS_SYNC_TOML_CHANGES: JSON.stringify([
            {
              program: "test_program",
              newId: "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
            },
          ]),
        },
        commands: [`${buildSyncCommand()} > "${outputPath}" 2>&1`],
      });

      const output = fs.readFileSync(outputPath, "utf8");
      expect(output).to.include("Syncing program ids for the configured cluster (localnet)");
      expect(output).to.include("Found incorrect program id declaration in programs/test-program/src/lib.rs");
      expect(output).to.include("Updated to aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x");
      expect(output).to.include("Found incorrect program id declaration in Anchor.toml for the program `test_program`");
      expect(output).to.include("All program id declarations are synced.");
      expect(output).to.include("Please rebuild the program to update the generated artifacts.");

      diffTest(scenario);
    });

    it("syncs only the specified program with --program-name flag", () => {
      const scenario = path.posix.join(KEYS_SYNC_ROOT, "specific-program");
      const { testDir } = setupTest(scenario);
      const outputPath = path.join(testDir, "output.txt");

      runCommands({
        cwd: path.join(testDir, "test-program"),
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_KEYS_SYNC_CLUSTER: "localnet",
          MOCK_KEYS_SYNC_CHANGES_MADE: "true",
          MOCK_KEYS_SYNC_SOURCE_CHANGES: JSON.stringify([
            {
              file: "programs/test-program/src/lib.rs",
              newId: "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
            },
          ]),
        },
        commands: [
          `${buildSyncCommand("--program-name test_program")} > "${outputPath}" 2>&1`,
        ],
      });

      const output = fs.readFileSync(outputPath, "utf8");
      expect(output).to.include("Syncing program ids for the configured cluster (localnet)");
      expect(output).to.include("Found incorrect program id declaration in programs/test-program/src/lib.rs");
      expect(output).to.include("Updated to aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x");
      expect(output).to.not.include("another_program");

      diffTest(scenario);
    });

    it("syncs multiple programs when all are out of sync", () => {
      const scenario = path.posix.join(KEYS_SYNC_ROOT, "multiple");
      const { testDir } = setupTest(scenario);
      const outputPath = path.join(testDir, "output.txt");

      runCommands({
        cwd: path.join(testDir, "test-program"),
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_KEYS_SYNC_CLUSTER: "localnet",
          MOCK_KEYS_SYNC_CHANGES_MADE: "true",
          MOCK_KEYS_SYNC_SOURCE_CHANGES: JSON.stringify([
            {
              file: "programs/test-program/src/lib.rs",
              newId: "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
            },
            {
              file: "programs/another-program/src/lib.rs",
              newId: "bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ",
            },
          ]),
        },
        commands: [`${buildSyncCommand()} > "${outputPath}" 2>&1`],
      });

      const output = fs.readFileSync(outputPath, "utf8");
      expect(output).to.include("Syncing program ids for the configured cluster (localnet)");
      expect(output).to.include("Found incorrect program id declaration in programs/test-program/src/lib.rs");
      expect(output).to.include("Updated to aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x");
      expect(output).to.include("Found incorrect program id declaration in programs/another-program/src/lib.rs");
      expect(output).to.include("Updated to bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ");
      expect(output).to.include("All program id declarations are synced.");

      diffTest(scenario);
    });
  });
}

