import fs from "fs";
import path from "path";
import { expect } from "chai";
import { diffTest, runCommands, setupTest, MOCK_BIN_DIR } from "@/lib";

const COMMAND_ROOT = "keys";
const SUBCOMMAND = "list";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");

function buildListCommand(): string {
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT} ${SUBCOMMAND}`;
}

const KEYS_LIST_ROOT = `${COMMAND_ROOT}/${SUBCOMMAND}`;

export function runKeysListTests() {
  describe(`${COMMAND_ROOT} ${SUBCOMMAND}`, () => {
    it("lists a single program's public key", () => {
      const scenario = path.posix.join(KEYS_LIST_ROOT, "basic");
      const { testDir } = setupTest(scenario);
      const outputPath = path.join(testDir, "output.txt");

      runCommands({
        cwd: path.join(testDir, "test-program"),
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_KEYS_LIST_PROGRAMS: JSON.stringify([
            {
              name: "test_program",
              pubkey: "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
            },
          ]),
        },
        commands: [`${buildListCommand()} > "${outputPath}" 2>&1`],
      });

      const output = fs.readFileSync(outputPath, "utf8").trim();
      expect(output).to.equal(
        "test_program: aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
      );

      diffTest(scenario);
    });

    it("lists multiple programs' public keys", () => {
      const scenario = path.posix.join(KEYS_LIST_ROOT, "multiple");
      const { testDir } = setupTest(scenario);
      const outputPath = path.join(testDir, "output.txt");

      runCommands({
        cwd: path.join(testDir, "test-program"),
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_KEYS_LIST_PROGRAMS: JSON.stringify([
            {
              name: "test_program",
              pubkey: "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
            },
            {
              name: "another_program",
              pubkey: "bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ",
            },
          ]),
        },
        commands: [`${buildListCommand()} > "${outputPath}" 2>&1`],
      });

      const output = fs.readFileSync(outputPath, "utf8").trim();
      const lines = output.split("\n");
      expect(lines).to.have.lengthOf(2);
      expect(lines[0]).to.equal(
        "test_program: aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
      );
      expect(lines[1]).to.equal(
        "another_program: bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ",
      );

      diffTest(scenario);
    });

    it("handles empty workspace with no programs", () => {
      const scenario = path.posix.join(KEYS_LIST_ROOT, "empty");
      const { testDir } = setupTest(scenario);
      const outputPath = path.join(testDir, "output.txt");

      runCommands({
        cwd: path.join(testDir, "test-program"),
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_KEYS_LIST_PROGRAMS: JSON.stringify([]),
        },
        commands: [`${buildListCommand()} > "${outputPath}" 2>&1`],
      });

      const output = fs.readFileSync(outputPath, "utf8").trim();
      expect(output).to.equal("");

      diffTest(scenario);
    });
  });
}

