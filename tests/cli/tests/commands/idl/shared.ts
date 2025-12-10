import fs from "fs";
import path from "path";
import { execSync } from "child_process";
import { MOCK_BIN_DIR, runCommands } from "@/lib";

export const IDL_TEST_ROOT = "idl";
export const IDL_BUILD_ROOT = path.posix.join(IDL_TEST_ROOT, "build");
export const IDL_INIT_ROOT = path.posix.join(IDL_TEST_ROOT, "init");
export const IDL_FETCH_ROOT = path.posix.join(IDL_TEST_ROOT, "fetch");
export const IDL_ERASE_AUTHORITY_ROOT = path.posix.join(
  IDL_TEST_ROOT,
  "erase-authority",
);
export const IDL_AUTHORITY_ROOT = path.posix.join(IDL_TEST_ROOT, "authority");
export const IDL_UPGRADE_ROOT = path.posix.join(IDL_TEST_ROOT, "upgrade");

export const TEST_KEYPAIR_PATH = path.resolve(
  __dirname,
  "../../..",
  "keypairs",
  "test-key.json",
);
export const LOCALNET_URL = "http://127.0.0.1:8899";
export const TEST_PROGRAM_ID =
  "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x";

export function getScenarioPath(...segments: string[]): string {
  return path.posix.join(IDL_TEST_ROOT, ...segments);
}

export function extractJsonFromOutput(output: string): any {
  const lines = output.split(/\r?\n/);
  const startIndex = lines.findIndex((line) => line.trim().startsWith("{"));

  if (startIndex === -1) {
    throw new Error("Failed to locate JSON payload in command output");
  }

  const jsonCandidate = lines.slice(startIndex).join("\n");
  const lastBraceIndex = jsonCandidate.lastIndexOf("}");
  const jsonPayload =
    lastBraceIndex === -1
      ? jsonCandidate
      : jsonCandidate.slice(0, lastBraceIndex + 1);

  return JSON.parse(jsonPayload);
}

export function readJsonFromMixedOutput(filePath: string): any {
  const contents = fs.readFileSync(filePath, "utf8");
  return extractJsonFromOutput(contents);
}

export function cleanupFile(filePath: string): void {
  if (fs.existsSync(filePath)) {
    fs.unlinkSync(filePath);
  }
}

export function cleanupWorkspaceArtifacts(
  workspaceDir: string,
  extraPaths: string[] = [],
): void {
  const entries = new Set<string>([
    "target",
    "app",
    ".anchor",
    "Cargo.lock",
    ...extraPaths,
  ]);

  entries.forEach((relativePath) => {
    if (!relativePath) return;
    const candidate = path.isAbsolute(relativePath)
      ? relativePath
      : path.join(workspaceDir, relativePath);

    if (!fs.existsSync(candidate)) return;

    const stat = fs.lstatSync(candidate);
    if (stat.isDirectory()) {
      fs.rmSync(candidate, { recursive: true, force: true });
    } else {
      fs.unlinkSync(candidate);
    }
  });
}

export interface ValidatorHandle {
  pid: number;
  pidFile: string;
  logPath: string;
}

export function startTestValidator(testDir: string): ValidatorHandle {
  const logPath = path.join(testDir, "validator.log");
  const pidFile = path.join(testDir, "validator.pid");

  runCommands({
    commands: [
      `SOLANA_TEST_VALIDATOR_PID_FILE="${pidFile}" solana-test-validator --reset > "${logPath}" 2>&1 &`,
    ],
    strictMode: false,
    prependPath: [MOCK_BIN_DIR],
  });

  const start = Date.now();
  while (!fs.existsSync(pidFile)) {
    if (Date.now() - start > 5_000) {
      throw new Error("Mock validator did not create PID file in time");
    }
    execSync("sleep 0.1");
  }

  runCommands({
    commands: ["sleep 8"],
    prependPath: [MOCK_BIN_DIR],
  });

  const pid = Number.parseInt(fs.readFileSync(pidFile, "utf8").trim(), 10);

  if (Number.isNaN(pid)) {
    throw new Error("Failed to start solana-test-validator");
  }

  return {
    pid,
    pidFile,
    logPath,
  };
}

export function stopTestValidator(handle: ValidatorHandle) {
  try {
    process.kill(handle.pid, "SIGTERM");
  } catch {
    // ignore - process might already have exited
  }

  cleanupFile(handle.pidFile);
  cleanupFile(handle.logPath);
}

export function createAnchorEnv(
  overrides: Record<string, string> = {},
): Record<string, string> {
  const base: Record<string, string> = {};

  for (const [key, value] of Object.entries(process.env)) {
    if (typeof value === "string") {
      base[key] = value;
    }
  }

  base.ANCHOR_PROVIDER_URL = LOCALNET_URL;
  base.ANCHOR_WALLET = TEST_KEYPAIR_PATH;
  const pathEntries = [MOCK_BIN_DIR];
  if (base.PATH) {
    pathEntries.push(base.PATH);
  }
  base.PATH = pathEntries.join(path.delimiter);

  for (const [key, value] of Object.entries(overrides)) {
    base[key] = value;
  }

  return base;
}

export function requestAirdrop(env: Record<string, string>): void {
  runCommands({
    commands: [
      `solana airdrop 2 --url ${LOCALNET_URL} --commitment confirmed --keypair "${TEST_KEYPAIR_PATH}"`,
    ],
    env,
    prependPath: [MOCK_BIN_DIR],
  });
}

