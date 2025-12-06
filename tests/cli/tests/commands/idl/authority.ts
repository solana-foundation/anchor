import fs from "fs";
import path from "path";
import { expect } from "chai";
import { diffTest, runCommands, setupTest, MOCK_BIN_DIR } from "@/lib";
import { IDL_AUTHORITY_ROOT } from "./shared";

const COMMAND_ROOT = "idl";
const SUBCOMMAND = "authority";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");

function buildAuthorityCommand(address: string): string {
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT} ${SUBCOMMAND} ${address}`;
}

export function runIdlAuthorityTests() {
  describe(`${COMMAND_ROOT} ${SUBCOMMAND}`, () => {
    it("prints authority for program id", () => {
      const scenario = path.posix.join(IDL_AUTHORITY_ROOT, "basic");
      const { testDir } = setupTest(scenario);
      const outputPath = path.join(testDir, "authority.txt");
      const address = "ProgramAuth11111111111111111111111111111111";
      const authority =
        "Auth1111111111111111111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_AUTHORITY_EXPECT_ADDRESS: address,
          MOCK_IDL_AUTHORITY_VALUE: authority,
        },
        commands: [`${buildAuthorityCommand(address)} > "${outputPath}"`],
      });

      const output = fs.readFileSync(outputPath, "utf8").trim();
      expect(output).to.equal(authority);

      diffTest(scenario);
    });

    it("prints authority for idl account address", () => {
      const scenario = path.posix.join(IDL_AUTHORITY_ROOT, "idl-account");
      const { testDir } = setupTest(scenario);
      const outputPath = path.join(testDir, "authority.txt");
      const idlAccount = "IdlAccountAuth111111111111111111111111111111";
      const authority =
        "Auth2222222222222222222222222222222222222222";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_AUTHORITY_EXPECT_ADDRESS: idlAccount,
          MOCK_IDL_AUTHORITY_VALUE: authority,
        },
        commands: [`${buildAuthorityCommand(idlAccount)} > "${outputPath}"`],
      });

      const output = fs.readFileSync(outputPath, "utf8").trim();
      expect(output).to.equal(authority);

      diffTest(scenario);
    });

    it("fails when authority lookup errors", () => {
      const scenario = path.posix.join(IDL_AUTHORITY_ROOT, "error");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "authority-error.log");
      const address = "ErrorProgramAuth111111111111111111111111111";
      let caught: Error | undefined;

      try {
        runCommands({
          cwd: testDir,
          prependPath: [MOCK_BIN_DIR],
          env: {
            MOCK_IDL_AUTHORITY_EXPECT_ADDRESS: address,
            MOCK_IDL_AUTHORITY_ERROR: "Mock authority failure",
          },
          commands: [`${buildAuthorityCommand(address)} > "${logPath}" 2>&1`],
        });
      } catch (err: any) {
        caught = err;
      }

      expect(caught, "idl authority should throw").to.be.instanceOf(Error);
      const logContents = fs.readFileSync(logPath, "utf8");
      expect(logContents).to.include("Mock authority failure");

      diffTest(scenario);
    });
  });
}

