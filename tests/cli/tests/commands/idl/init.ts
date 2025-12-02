import fs from "fs";
import path from "path";
import { expect } from "chai";
import { runCommands, setupTest, MOCK_BIN_DIR } from "@/lib";
import {
  IDL_INIT_ROOT,
  TEST_PROGRAM_ID,
  cleanupFile,
  cleanupWorkspaceArtifacts,
  createAnchorEnv,
  requestAirdrop,
  startTestValidator,
  stopTestValidator,
} from "./shared";

const LONG_TEST_TIMEOUT_MS = 120_000;

const DEFAULT_IDL_JSON = {
  version: "0.1.0",
  name: "test_program",
  instructions: [
    {
      name: "initialize",
      accounts: [],
      args: [],
    },
  ],
  metadata: {
    address: TEST_PROGRAM_ID,
  },
};

function ensureIdlFile(workspaceDir: string, programName = "test_program"): string {
  const idlDir = path.join(workspaceDir, "target", "idl");
  const idlPath = path.join(idlDir, `${programName}.json`);
  if (!fs.existsSync(idlPath)) {
    fs.mkdirSync(idlDir, { recursive: true });
    const idl = { ...DEFAULT_IDL_JSON, name: programName };
    fs.writeFileSync(idlPath, JSON.stringify(idl, null, 2));
  }
  return idlPath;
}

export function runIdlInitTests() {
  describe("init", () => {
    it("initializes the IDL account with default filepath", function () {
      this.timeout(LONG_TEST_TIMEOUT_MS);

      const scenario = path.posix.join(IDL_INIT_ROOT, "basic");
      const { testDir } = setupTest(scenario);
      const workspaceDir = path.join(testDir, "test-program");
      const env = createAnchorEnv();
    const anchorBin = path.join(MOCK_BIN_DIR, "anchor");
      const validator = startTestValidator(testDir);
      const initLogPath = path.join(testDir, "idl-init.log");

      try {
        requestAirdrop(env);

        runCommands({
          cwd: workspaceDir,
          env,
          commands: [`${anchorBin} build`],
          prependPath: [MOCK_BIN_DIR],
        });

        const defaultIdlPath = ensureIdlFile(workspaceDir);

        runCommands({
          cwd: workspaceDir,
          env,
          commands: [`${anchorBin} deploy --no-idl`],
          prependPath: [MOCK_BIN_DIR],
        });

        runCommands({
          cwd: workspaceDir,
          env,
          commands: [
            `${anchorBin} idl init --filepath "${defaultIdlPath}" ${TEST_PROGRAM_ID} > "${initLogPath}" 2>&1`,
          ],
          prependPath: [MOCK_BIN_DIR],
        });

        const output = fs.readFileSync(initLogPath, "utf8");
        expect(output).to.include("Idl account created:");
      } finally {
        stopTestValidator(validator);
        cleanupFile(initLogPath);
        cleanupWorkspaceArtifacts(workspaceDir);
      }
    });

    it("accepts a custom filepath for the IDL JSON", function () {
      this.timeout(LONG_TEST_TIMEOUT_MS);

      const scenario = path.posix.join(IDL_INIT_ROOT, "custom-filepath");
      const { testDir } = setupTest(scenario);
      const workspaceDir = path.join(testDir, "test-program");
    const env = createAnchorEnv();
    const anchorBin = path.join(MOCK_BIN_DIR, "anchor");
      const validator = startTestValidator(testDir);
      const initLogPath = path.join(testDir, "idl-init-custom.log");

      try {
        requestAirdrop(env);

        runCommands({
          cwd: workspaceDir,
          env,
          commands: [`${anchorBin} build`],
          prependPath: [MOCK_BIN_DIR],
        });

        const defaultIdlPath = ensureIdlFile(workspaceDir);

        const customDir = path.join(testDir, "custom-idl");
        const customPath = path.join(customDir, "custom.json");
        fs.mkdirSync(customDir, { recursive: true });
        fs.copyFileSync(defaultIdlPath, customPath);

        runCommands({
          cwd: workspaceDir,
          env,
          commands: [`${anchorBin} deploy --no-idl`],
          prependPath: [MOCK_BIN_DIR],
        });

        runCommands({
          cwd: workspaceDir,
          env,
          commands: [
            `${anchorBin} idl init --filepath "${customPath}" ${TEST_PROGRAM_ID} > "${initLogPath}" 2>&1`,
          ],
          prependPath: [MOCK_BIN_DIR],
        });

        const output = fs.readFileSync(initLogPath, "utf8");
        expect(output).to.include("Idl account created:");
      } finally {
        stopTestValidator(validator);
        cleanupFile(initLogPath);
        cleanupWorkspaceArtifacts(workspaceDir, [
          path.join(testDir, "custom-idl"),
        ]);
      }
    });

    it("fails when the IDL filepath does not exist", function () {
      this.timeout(LONG_TEST_TIMEOUT_MS);

      const scenario = path.posix.join(IDL_INIT_ROOT, "missing-file");
      const { testDir } = setupTest(scenario);
      const workspaceDir = path.join(testDir, "test-program");
    const env = createAnchorEnv();
    const anchorBin = path.join(MOCK_BIN_DIR, "anchor");
      const validator = startTestValidator(testDir);

      try {
        requestAirdrop(env);

        runCommands({
          cwd: workspaceDir,
          env,
          commands: [`${anchorBin} build`],
          prependPath: [MOCK_BIN_DIR],
        });

        ensureIdlFile(workspaceDir);

        runCommands({
          cwd: workspaceDir,
          env,
          commands: [`${anchorBin} deploy --no-idl`],
          prependPath: [MOCK_BIN_DIR],
        });

        const missingPath = path.join(testDir, "missing-idl.json");

        expect(() =>
          runCommands({
            cwd: workspaceDir,
            env,
            commands: [
              `${anchorBin} idl init --filepath "${missingPath}" ${TEST_PROGRAM_ID}`,
            ],
            prependPath: [MOCK_BIN_DIR],
          }),
        ).to.throw(/IDL doesn't exist|No such file or directory|ENOENT/);
      } finally {
        stopTestValidator(validator);
        cleanupWorkspaceArtifacts(workspaceDir);
      }
    });
  });
}
