import fs from "fs";
import path from "path";
import { expect } from "chai";
import { diffTest, runCommands, setupTest, MOCK_BIN_DIR } from "@/lib";
import { IDL_ERASE_AUTHORITY_ROOT } from "./shared";

const COMMAND_ROOT = "idl";
const SUBCOMMAND = "erase-authority";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");

function buildEraseCommand(programId: string, extraArgs = ""): string {
  const suffix = extraArgs ? ` ${extraArgs}` : "";
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT} ${SUBCOMMAND} -p ${programId}${suffix}`;
}

export function runIdlEraseAuthorityTests() {
  describe(`${COMMAND_ROOT} ${SUBCOMMAND}`, () => {
    it("erases authority after confirmation", () => {
      const scenario = path.posix.join(IDL_ERASE_AUTHORITY_ROOT, "success");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "erase-authority.log");
      const programId = "EraseAuth11111111111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_ERASE_EXPECT_ADDRESS: programId,
          MOCK_IDL_ERASE_CONFIRM: "y",
        },
        commands: [`${buildEraseCommand(programId)} > "${logPath}" 2>&1`],
      });

      const output = fs.readFileSync(logPath, "utf8");
      expect(output).to.include("Are you sure you want to erase the IDL authority");
      expect(output).to.include("Authority update complete.");

      diffTest(scenario);
    });

    it("cancels when user declines", () => {
      const scenario = path.posix.join(IDL_ERASE_AUTHORITY_ROOT, "decline");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "erase-authority.log");
      const programId = "EraseDecline111111111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_ERASE_EXPECT_ADDRESS: programId,
          MOCK_IDL_ERASE_CONFIRM: "n",
        },
        commands: [`${buildEraseCommand(programId)} > "${logPath}" 2>&1`],
      });

      const output = fs.readFileSync(logPath, "utf8");
      expect(output).to.include("Are you sure you want to erase the IDL authority");
      expect(output).to.include("Not erasing.");

      diffTest(scenario);
    });

    it("fails when erase authority errors", () => {
      const scenario = path.posix.join(IDL_ERASE_AUTHORITY_ROOT, "error");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "erase-authority-error.log");
      const programId = "EraseError1111111111111111111111111111111";
      let caught: Error | undefined;

      try {
        runCommands({
          cwd: testDir,
          prependPath: [MOCK_BIN_DIR],
          env: {
            MOCK_IDL_ERASE_EXPECT_ADDRESS: programId,
            MOCK_IDL_ERASE_CONFIRM: "y",
            MOCK_IDL_ERASE_ERROR: "Mock erase failure",
          },
          commands: [`${buildEraseCommand(programId)} > "${logPath}" 2>&1`],
        });
      } catch (err: any) {
        caught = err;
      }

      expect(caught, "idl erase-authority should throw").to.be.instanceOf(Error);
      const output = fs.readFileSync(logPath, "utf8");
      expect(output).to.include("Mock erase failure");

      diffTest(scenario);
    });

    it("accepts priority fee flag", () => {
      const scenario = path.posix.join(IDL_ERASE_AUTHORITY_ROOT, "priority-fee");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "erase-authority.log");
      const programId = "ErasePriority11111111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_ERASE_EXPECT_ADDRESS: programId,
          MOCK_IDL_ERASE_CONFIRM: "y",
        },
        commands: [
          `${buildEraseCommand(programId, "--priority-fee 9000")} > "${logPath}" 2>&1`,
        ],
      });

      const output = fs.readFileSync(logPath, "utf8");
      expect(output).to.include("Using priority fee: 9000");
      expect(output).to.include("Authority update complete.");

      diffTest(scenario);
    });
  });
}

