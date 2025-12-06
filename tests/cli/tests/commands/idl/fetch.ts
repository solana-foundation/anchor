import fs from "fs";
import path from "path";
import { expect } from "chai";
import { diffTest, runCommands, setupTest, MOCK_BIN_DIR } from "@/lib";
import { IDL_FETCH_ROOT } from "./shared";

const COMMAND_ROOT = "idl";
const SUBCOMMAND = "fetch";
const ANCHOR_BIN = path.join(MOCK_BIN_DIR, "anchor");

function buildFetchCommand(address: string, extraArgs = ""): string {
  const suffix = extraArgs ? ` ${extraArgs}` : "";
  return `"${ANCHOR_BIN}" ${COMMAND_ROOT} ${SUBCOMMAND} ${address}${suffix}`;
}

function fixturePath(filename: string): string {
  return path.join(__dirname, "fixtures", "fetch", filename);
}

export function runIdlFetchTests() {
  describe(`${COMMAND_ROOT} ${SUBCOMMAND}`, () => {
    it("fetches IDL JSON to stdout by default", () => {
      const scenario = path.posix.join(IDL_FETCH_ROOT, "basic");
      const { testDir } = setupTest(scenario);
      const stdoutPath = path.join(testDir, "fetch-output.json");
      const address = "BasicProgram111111111111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_FETCH_SOURCE: fixturePath("basic-idl.json"),
          MOCK_IDL_FETCH_EXPECT_ADDRESS: address,
        },
        commands: [
          `${buildFetchCommand(address)} > "${stdoutPath}" 2>&1`,
        ],
      });

      const output = JSON.parse(fs.readFileSync(stdoutPath, "utf8"));
      expect(output.name).to.equal("basic_program");
      expect(output.metadata?.address).to.equal("BasicAddress1111111111111111111111111111111");

      diffTest(scenario);
    });

    it("writes IDL JSON to the provided output path", () => {
      const scenario = path.posix.join(IDL_FETCH_ROOT, "out-file");
      const { testDir } = setupTest(scenario);
      const artifactsDir = path.join(testDir, "artifacts");
      const outPath = path.join(artifactsDir, "fetched-idl.json");
      const address = "OutFileProgram1111111111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_FETCH_SOURCE: fixturePath("out-file-idl.json"),
          MOCK_IDL_FETCH_EXPECT_ADDRESS: address,
        },
        commands: [buildFetchCommand(address, `--out "${outPath}"`)],
      });

      expect(fs.existsSync(outPath)).to.equal(true);
      const fetched = JSON.parse(fs.readFileSync(outPath, "utf8"));
      expect(fetched.name).to.equal("out_file_program");
      expect(fetched.metadata?.address).to.equal(address);

      diffTest(scenario);
    });

    it("handles legacy IDL formats", () => {
      const scenario = path.posix.join(IDL_FETCH_ROOT, "legacy");
      const { testDir } = setupTest(scenario);
      const stdoutPath = path.join(testDir, "fetch-output.json");
      const address = "LegacyProgram1111111111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_FETCH_SOURCE: fixturePath("legacy-idl.json"),
          MOCK_IDL_FETCH_EXPECT_ADDRESS: address,
        },
        commands: [
          `${buildFetchCommand(address)} > "${stdoutPath}" 2>&1`,
        ],
      });

      const output = JSON.parse(fs.readFileSync(stdoutPath, "utf8"));
      expect(output.version).to.equal("0.0.0");
      expect(output.metadata?.address).to.equal(address);
      expect(output.instructions?.[0]?.name).to.equal("initialize");

      diffTest(scenario);
    });

    it("supports fetching by IDL account address", () => {
      const scenario = path.posix.join(IDL_FETCH_ROOT, "idl-account");
      const { testDir } = setupTest(scenario);
      const stdoutPath = path.join(testDir, "fetch-output.json");
      const address = "IdlAccount111111111111111111111111111111111";

      runCommands({
        cwd: testDir,
        prependPath: [MOCK_BIN_DIR],
        env: {
          MOCK_IDL_FETCH_SOURCE: fixturePath("idl-account-idl.json"),
          MOCK_IDL_FETCH_EXPECT_ADDRESS: address,
        },
        commands: [
          `${buildFetchCommand(address)} > "${stdoutPath}" 2>&1`,
        ],
      });

      const output = JSON.parse(fs.readFileSync(stdoutPath, "utf8"));
      expect(output.name).to.equal("idl_account_program");
      expect(output.metadata?.address).to.equal("ProgramDerived111111111111111111111111111111");

      diffTest(scenario);
    });

    it("fails when the fetch operation encounters an error", () => {
      const scenario = path.posix.join(IDL_FETCH_ROOT, "error");
      const { testDir } = setupTest(scenario);
      const logPath = path.join(testDir, "fetch-error.log");
      const address = "ErrorProgram11111111111111111111111111111111";
      let caught: Error | undefined;

      try {
        runCommands({
          cwd: testDir,
          prependPath: [MOCK_BIN_DIR],
          env: {
            MOCK_IDL_FETCH_ERROR: "Mock fetch failure",
            MOCK_IDL_FETCH_EXPECT_ADDRESS: address,
          },
          commands: [
            `${buildFetchCommand(address)} > "${logPath}" 2>&1`,
          ],
        });
      } catch (err: any) {
        caught = err;
      }

      expect(caught, "idl fetch should throw").to.be.instanceOf(Error);
      const logContents = fs.readFileSync(logPath, "utf8");
      expect(logContents).to.include("Mock fetch failure");

      diffTest(scenario);
    });
  });
}

