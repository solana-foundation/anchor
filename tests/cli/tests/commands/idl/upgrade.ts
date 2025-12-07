import fs from "fs";
import path from "path";
import { expect } from "chai";
import { diffTest, runCommands, setupTest, MOCK_BIN_DIR } from "@/lib";
import { IDL_UPGRADE_ROOT } from "./shared";

const COMMAND_ROOT = "idl";
const SUBCOMMAND = "upgrade";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");

function buildUpgradeCommand(
  programId: string,
  filepath: string,
  extraArgs = "",
): string {
  const suffix = extraArgs ? ` ${extraArgs}` : "";
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT} ${SUBCOMMAND} --filepath "${filepath}" ${programId}${suffix}`;
}

function fixturePath(filename: string): string {
  return path.join(__dirname, "fixtures", "upgrade", filename);
}

export function runIdlUpgradeTests() {
  describe(`${COMMAND_ROOT} ${SUBCOMMAND}`, () => {
    it("upgrades IDL with default filepath", () => {
      const scenario = path.posix.join(IDL_UPGRADE_ROOT, "basic");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "upgrade.log");
      const programId = "UpgradeProgram111111111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_UPGRADE_EXPECT_PROGRAM_ID: programId,
          MOCK_IDL_UPGRADE_IDL_ADDRESS:
            "IdlUpgradeSuccess11111111111111111111111111111",
        },
        commands: [
          `${buildUpgradeCommand(programId, fixturePath("basic-idl.json"))} > "${logPath}" 2>&1`,
        ],
      });

      const output = fs.readFileSync(logPath, "utf8").trim();
      expect(output).to.equal(
        "Idl account IdlUpgradeSuccess11111111111111111111111111111 successfully upgraded",
      );
      
      fs.writeFileSync(path.join(testDir, ".gitkeep"), "\n\n");

      diffTest(scenario);
    });

    it("supports custom filepath", () => {
      const scenario = path.posix.join(IDL_UPGRADE_ROOT, "custom-filepath");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "upgrade.log");
      const programId = "CustomUpgradeProgram1111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_UPGRADE_EXPECT_PROGRAM_ID: programId,
          MOCK_IDL_UPGRADE_IDL_ADDRESS:
            "CustomUpgrade111111111111111111111111111",
        },
        commands: [
          `${buildUpgradeCommand(
            programId,
            fixturePath("custom-filepath-idl.json"),
          )} > "${logPath}" 2>&1`,
        ],
      });

      const output = fs.readFileSync(logPath, "utf8").trim();
      expect(output).to.equal(
        "Idl account CustomUpgrade111111111111111111111111111 successfully upgraded",
      );
      
      fs.writeFileSync(path.join(testDir, ".gitkeep"), "\n\n");

      diffTest(scenario);
    });

    it("accepts priority fee flag", () => {
      const scenario = path.posix.join(IDL_UPGRADE_ROOT, "priority-fee");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "upgrade.log");
      const programId = "PriorityUpgradeProgram111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_UPGRADE_EXPECT_PROGRAM_ID: programId,
          MOCK_IDL_UPGRADE_IDL_ADDRESS:
            "PriorityUpgrade11111111111111111111111111",
        },
        commands: [
          `${buildUpgradeCommand(
            programId,
            fixturePath("priority-fee-idl.json"),
            "--priority-fee 9000",
          )} > "${logPath}" 2>&1`,
        ],
      });

      const output = fs.readFileSync(logPath, "utf8");
      expect(output).to.include("Using priority fee: 9000");
      expect(output).to.include(
        "Idl account PriorityUpgrade11111111111111111111111111 successfully upgraded",
      );
      
      fs.writeFileSync(path.join(testDir, ".gitkeep"), "\n\n");

      diffTest(scenario);
    });

    it("fails when filepath does not exist", () => {
      const scenario = path.posix.join(IDL_UPGRADE_ROOT, "error");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "upgrade-error.log");
      const programId = "UpgradeError111111111111111111111111111111";
      let caught: Error | undefined;

      try {
        runCommands({
          cwd: testDir,
          prependPath: [MOCK_BIN_DIR],
          env: {
            MOCK_IDL_UPGRADE_EXPECT_PROGRAM_ID: programId,
          },
          commands: [
            `${buildUpgradeCommand(
              programId,
              path.join(testDir, "missing-idl.json"),
            )} > "${logPath}" 2>&1`,
          ],
        });
      } catch (err: any) {
        caught = err;
      }

      expect(caught, "idl upgrade should throw").to.be.instanceOf(Error);
      const output = fs.readFileSync(logPath, "utf8");
      expect(output).to.include("Error: IDL doesn't exist");

      diffTest(scenario);
    });
  });
}


