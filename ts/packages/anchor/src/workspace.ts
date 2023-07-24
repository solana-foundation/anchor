import * as toml from "toml";
import { snakeCase } from "snake-case";
import { Program } from "./program/index.js";
import { isBrowser } from "./utils/common.js";

/**
 * The `workspace` namespace provides a convenience API to automatically
 * search for and deserialize [[Program]] objects defined by compiled IDLs
 * in an Anchor workspace.
 *
 * This API is for Node only.
 */
const workspace = new Proxy(
  {},
  {
    get(workspaceCache: { [key: string]: Program }, programName: string) {
      if (isBrowser) {
        throw new Error("Workspaces aren't available in the browser");
      }

      // Converting `programName` to snake_case enables the ability to use any
      // of the following to access the workspace program:
      // `workspace.myProgram`, `workspace.MyProgram`, `workspace["my-program"]`...
      programName = snakeCase(programName);

      // Return early if the program is in cache
      if (workspaceCache[programName]) return workspaceCache[programName];

      const fs = require("fs");
      const path = require("path");

      const idlFolder = path.join("target", "idl");
      if (!fs.existsSync(idlFolder)) {
        throw new Error(
          `${idlFolder} doesn't exist. Did you run \`anchor build\`?`
        );
      }

      // Override the workspace programs if the user put them in the config.
      const anchorToml = toml.parse(fs.readFileSync("Anchor.toml"));
      const clusterId = anchorToml.provider.cluster;
      const programEntry = anchorToml.programs?.[clusterId]?.[programName];

      let idlPath;
      let programId;
      if (typeof programEntry === "object" && programEntry.idl) {
        idlPath = programEntry.idl;
        programId = programEntry.address;
      } else {
        idlPath = path.join(idlFolder, `${programName}.json`);
      }

      const idl = JSON.parse(fs.readFileSync(idlPath));
      if (!programId) {
        if (!idl.metadata?.address) {
          throw new Error(
            `IDL for program \`${programName}\` does not have \`metadata.address\` field.\n` +
              "To add the missing field, run `anchor deploy` or `anchor test`."
          );
        }
        programId = idl.metadata.address;
      }
      workspaceCache[programName] = new Program(idl, programId);

      return workspaceCache[programName];
    },
  }
);

export default workspace;
