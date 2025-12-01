import fs from "fs";
import path from "path";

export const IDL_TEST_ROOT = "idl";
export const IDL_BUILD_ROOT = path.posix.join(IDL_TEST_ROOT, "build");

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


