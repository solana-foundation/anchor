import camelCase from "camelcase";
import * as toml from "toml";
import { PublicKey } from "@solana/web3.js";
import { Program } from "./program";
import { getProvider } from "./";

let _populatedWorkspace = false;

/**
 * The `workspace` namespace provides a convenience API to automatically
 * search for and deserialize [[Program]] objects defined by compiled IDLs
 * in an Anchor workspace.
 *
 * This API is for Node only.
 */
const workspace = new Proxy({} as any, {
  get(workspaceCache: { [key: string]: Program }, programName: string) {
    const find = require("find");
    const fs = require("fs");
    const process = require("process");

    if (typeof window !== "undefined") {
      // Workspaces aren't available in the browser, yet.
      return undefined;
    }

    if (!_populatedWorkspace) {
      const path = require("path");

      let projectRoot = process.cwd();
      while (!fs.existsSync(path.join(projectRoot, "Anchor.toml"))) {
        const parentDir = path.dirname(projectRoot);
        if (parentDir === projectRoot) {
          projectRoot = undefined;
        }
        projectRoot = parentDir;
      }

      if (projectRoot === undefined) {
        throw new Error("Could not find workspace root.");
      }

      find
        .fileSync(/target\/idl\/.*\.json/, projectRoot)
        .reduce((programs: any, path: string) => {
          const idlStr = fs.readFileSync(path);
          const idl = JSON.parse(idlStr);
          const name = camelCase(idl.name, { pascalCase: true });
          if (idl.metadata && idl.metadata.address) {
            programs[name] = new Program(
              idl,
              new PublicKey(idl.metadata.address)
            );
          }
          return programs;
        }, workspaceCache);

      // Override the workspace programs if the user put them in the config.
      const anchorToml = toml.parse(
        fs.readFileSync(path.join(projectRoot, "Anchor.toml"), "utf-8")
      );
      const clusterId = anchorToml.provider.cluster;
      if (anchorToml.clusters && anchorToml.clusters[clusterId]) {
        attachWorkspaceOverride(workspaceCache, anchorToml.clusters[clusterId]);
      }

      _populatedWorkspace = true;
    }

    return workspaceCache[programName];
  },
});

function attachWorkspaceOverride(
  workspaceCache: { [key: string]: Program },
  overrideConfig: { [key: string]: string }
) {
  Object.keys(overrideConfig).forEach((programName) => {
    const wsProgramName = camelCase(programName, { pascalCase: true });
    const oldProgram = workspaceCache[wsProgramName];
    const overrideAddress = new PublicKey(overrideConfig[programName]);
    workspaceCache[wsProgramName] = new Program(
      oldProgram.idl,
      overrideAddress
    );
  });
}

export default workspace;
