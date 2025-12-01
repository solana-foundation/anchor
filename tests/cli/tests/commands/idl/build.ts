import fs from "fs";
import path from "path";
import { expect } from "chai";
import { anchorCommand, diffTest, runCommands, setupTest } from "@/lib";
import {
  IDL_BUILD_ROOT,
  cleanupFile,
  readJsonFromMixedOutput,
} from "./shared";

const LONG_TEST_TIMEOUT_MS = 120_000;

export function runIdlBuildTests() {
  describe("build", () => {
    it("emits IDL JSON to stdout by default", function () {
      this.timeout(LONG_TEST_TIMEOUT_MS);
      const scenario = path.posix.join(IDL_BUILD_ROOT, "basic");
      const { testDir } = setupTest(scenario);
      const workspaceDir = path.join(testDir, "test-program");
      const stdoutPath = path.join(testDir, "idl-stdout.log");

      runCommands({
        cwd: workspaceDir,
        commands: [
          `${anchorCommand("idl build")} > "${stdoutPath}" 2>&1`,
        ],
      });

      try {
        const idl = readJsonFromMixedOutput(stdoutPath);
        expect(idl.metadata?.name).to.equal("test_program");
        const instructionNames = (idl.instructions ?? []).map(
          (instruction: { name: string }) => instruction.name,
        );
        expect(instructionNames).to.deep.equal(["initialize"]);
      } finally {
        cleanupFile(stdoutPath);
      }

      cleanupWorkspaceArtifacts(workspaceDir);
      diffTest(scenario);
    });

    it("writes IDL JSON to the provided output path", function () {
      this.timeout(LONG_TEST_TIMEOUT_MS);
      const scenario = path.posix.join(IDL_BUILD_ROOT, "out-file");
      const { testDir } = setupTest(scenario);
      const workspaceDir = path.join(testDir, "test-program");
      const artifactsDir = path.join(testDir, "artifacts");
      const outPath = path.join(artifactsDir, "custom-idl.json");

      fs.mkdirSync(artifactsDir, { recursive: true });

      runCommands({
        cwd: workspaceDir,
        commands: [
          anchorCommand(`idl build --out "${outPath}"`),
        ],
      });

      try {
        expect(fs.existsSync(outPath)).to.equal(true);
        const idl = JSON.parse(fs.readFileSync(outPath, "utf8"));
        expect(idl.metadata?.name).to.equal("test_program");
      } finally {
        cleanupFile(outPath);
        fs.rmSync(artifactsDir, { recursive: true, force: true });
      }

      cleanupWorkspaceArtifacts(workspaceDir);
      diffTest(scenario);
    });

    it("writes TypeScript definitions when --out-ts is provided", function () {
      this.timeout(LONG_TEST_TIMEOUT_MS);
      const scenario = path.posix.join(IDL_BUILD_ROOT, "out-ts");
      const { testDir } = setupTest(scenario);
      const workspaceDir = path.join(testDir, "test-program");

      const artifactsDir = path.join(testDir, "artifacts");
      const outJsonPath = path.join(artifactsDir, "idl.json");
      const outTsPath = path.join(artifactsDir, "types.ts");

      fs.mkdirSync(artifactsDir, { recursive: true });

      runCommands({
        cwd: workspaceDir,
        commands: [
          anchorCommand(
            `idl build --out "${outJsonPath}" --out-ts "${outTsPath}"`,
          ),
        ],
      });

      try {
        expect(fs.existsSync(outJsonPath)).to.equal(true);
        expect(fs.existsSync(outTsPath)).to.equal(true);

        const idl = JSON.parse(fs.readFileSync(outJsonPath, "utf8"));
        expect(idl.metadata?.name).to.equal("test_program");

        const types = fs.readFileSync(outTsPath, "utf8");
        expect(types).to.contain("export type TestProgram");
      } finally {
        cleanupFile(outJsonPath);
        cleanupFile(outTsPath);
        fs.rmSync(artifactsDir, { recursive: true, force: true });
      }

      cleanupWorkspaceArtifacts(workspaceDir);
      diffTest(scenario);
    });

    it("supports selecting a program by name in multi-program workspaces", function () {
      this.timeout(LONG_TEST_TIMEOUT_MS);
      const scenario = path.posix.join(IDL_BUILD_ROOT, "program-name");
      const { testDir } = setupTest(scenario);
      const workspaceDir = path.join(testDir, "test-program");
      const stdoutPath = path.join(testDir, "idl-program.log");

      runCommands({
        cwd: workspaceDir,
        commands: [
          `${anchorCommand(
            "idl build --program-name another-program",
          )} > "${stdoutPath}" 2>&1`,
        ],
      });

      try {
        const idl = readJsonFromMixedOutput(stdoutPath);
        expect(idl.metadata?.name).to.equal("another_program");
        expect(idl.instructions).to.be.an("array").that.is.not.empty;
      } finally {
        cleanupFile(stdoutPath);
      }

      cleanupWorkspaceArtifacts(workspaceDir);
      diffTest(scenario);
    });

    it("fails when the requested program does not exist", function () {
      this.timeout(LONG_TEST_TIMEOUT_MS);
      const scenario = path.posix.join(IDL_BUILD_ROOT, "basic");
      const { testDir } = setupTest(scenario);
      const workspaceDir = path.join(testDir, "test-program");

      expect(() =>
        runCommands({
          cwd: workspaceDir,
          commands: [
            anchorCommand("idl build --program-name missing-program"),
          ],
        }),
      ).to.throw(/Program missing-program not found/);
    });
  });
}

function cleanupWorkspaceArtifacts(workspaceDir: string): void {
  const removeIfExists = (relativePath: string) => {
    const absolutePath = path.join(workspaceDir, relativePath);
    if (fs.existsSync(absolutePath)) {
      fs.rmSync(absolutePath, { recursive: true, force: true });
    }
  };

  removeIfExists("target");
  removeIfExists("app");
  removeIfExists(".anchor");
  removeIfExists("Cargo.lock");
}

