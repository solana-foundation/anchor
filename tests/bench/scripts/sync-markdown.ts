/** Sync Markdown files in /bench based on the data from bench.json */

import { BenchData, Markdown } from "./utils";

(async () => {
  const bench = await BenchData.open();

  await BenchData.forEachMarkdown((markdown, fileName) => {
    if (fileName === "COMPUTE_UNITS.md") {
      const versions = bench.getVersions();

      // On the first version, compare with itself to update it with no changes
      versions.unshift(versions[0]);

      for (const i in versions) {
        const currentVersion = versions[i];
        const nextVersion = versions[+i + 1];

        if (currentVersion === "unreleased") {
          return;
        }

        const newData = bench.get(nextVersion);
        const oldData = bench.get(currentVersion);

        // Create table
        const table = Markdown.createTable(
          "Instruction",
          "Compute Units",
          "+/-"
        );

        bench.compareComputeUnits(
          newData.result.computeUnits,
          oldData.result.computeUnits,
          ({ ixName, newComputeUnits, oldComputeUnits }) => {
            if (newComputeUnits === null) {
              // Deleted instruction
              return;
            }

            let changeText;
            if (oldComputeUnits === null) {
              // New instruction
              changeText = "N/A";
            } else {
              const delta = newComputeUnits - oldComputeUnits;
              const percentChange = (
                (newComputeUnits / oldComputeUnits - 1) *
                100
              ).toFixed(2);

              if (+percentChange > 0) {
                changeText = `🔴 **+${delta} (${percentChange}%)**`;
              } else {
                changeText = `🟢 **${delta} (${percentChange.slice(1)}%)**`;
              }
            }

            table.insert(ixName, newComputeUnits.toString(), changeText);
          },
          (ixName, computeUnits) => {
            table.insert(
              ixName,
              computeUnits.toString(),
              +i === 0 ? "N/A" : "-"
            );
          }
        );

        // Update version data
        markdown.updateVersion({
          version: nextVersion,
          solanaVersion: newData.solanaVersion,
          table,
        });
      }
    }
  });
})();
