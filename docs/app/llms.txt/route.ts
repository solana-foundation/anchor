import { docsSource } from "@/app/source";
import type { Node } from "fumadocs-core/page-tree";

export const revalidate = false;

const BASE_URL = "https://www.anchor-lang.com";

const HEADER = `# Anchor Framework

> Anchor is the leading development framework for building secure Solana programs (smart contracts), simplifying the process of writing, testing, deploying, and interacting with on-chain programs.

This documentation covers the Anchor framework for Solana program development — from installation and core concepts to advanced features and reference material.

## AI Agent Resources

- [Solana Dev Skill](https://solana.com/SKILL.md): Comprehensive guide for AI agents to understand and build on Solana
- [Full Anchor Documentation](${BASE_URL}/llms-full.txt): Complete inline documentation for all Anchor pages
`;

// Pages under these URL prefixes go to Optional
const OPTIONAL_PREFIXES = ["/docs/updates"];

function isOptionalUrl(url: string): boolean {
  return OPTIONAL_PREFIXES.some((prefix) => url.startsWith(prefix));
}

function pageLink(url: string): string {
  const page = docsSource.getPages().find((p) => p.url === url);
  if (!page) return "";
  const title = page.data.title ?? "";
  const desc = page.data.description ?? "";
  const absUrl = `${BASE_URL}${url}`;
  return desc
    ? `- [${title}](${absUrl}): ${desc}`
    : `- [${title}](${absUrl})`;
}

function collectPages(node: Node, main: string[], optional: string[]) {
  switch (node.type) {
    case "separator": {
      const name = typeof node.name === "string" ? node.name : "";
      if (name) main.push(`\n## ${name}\n`);
      break;
    }
    case "page": {
      const line = pageLink(node.url);
      if (!line) break;
      if (isOptionalUrl(node.url)) {
        optional.push(line);
      } else {
        main.push(line);
      }
      break;
    }
    case "folder": {
      if (node.index) {
        collectPages(node.index, main, optional);
      }
      for (const child of node.children) {
        collectPages(child, main, optional);
      }
      break;
    }
  }
}

export function GET() {
  const tree = docsSource.pageTree;
  const main: string[] = [];
  const optional: string[] = [];

  for (const child of tree.children) {
    collectPages(child, main, optional);
  }

  const parts = [HEADER, ...main];
  if (optional.length > 0) {
    parts.push("\n## Optional\n");
    parts.push(...optional);
  }

  return new Response(parts.join("\n").trim() + "\n", {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
    },
  });
}
