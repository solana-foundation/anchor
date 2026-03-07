import { NextResponse } from 'next/server';
import { extractTextFromMDX } from '../api/llms-txt/ast-extract';
import path from 'path';
import { readFile, readdir } from 'fs/promises';

export async function GET() {
  try {
    const docsDir = path.join(process.cwd(), 'content/docs');

    // Find all MDX files recursively
    const mdxFiles = await findMdxFiles(docsDir);

    let content = '# Anchor Documentation for LLMs\n\n';
    content += 'This is a text-only version of the Anchor documentation, optimized for Large Language Model consumption.\n\n';
    content += 'Anchor is a framework for building secure Solana programs (smart contracts).\n\n';

    // Sort files to maintain consistent order
    const sortedFiles = mdxFiles.sort((a, b) => a.localeCompare(b));

    for (const file of sortedFiles) {
      try {
        const mdxContent = await readFile(file, 'utf-8');

        // Extract clean text
        const extractedText = await extractTextFromMDX(mdxContent);

        if (extractedText.trim()) {
          // Create a readable title from the file path
          const title = createTitleFromPath(path.relative(docsDir, file));
          content += `## ${title}\n\n`;
          content += extractedText.trim() + '\n\n';
          content += '---\n\n';
        }
      } catch (error) {
        console.warn(`Failed to process ${file}:`, error);
      }
    }

    return new NextResponse(content, {
      headers: {
        'Content-Type': 'text/plain; charset=utf-8',
        'Cache-Control': 'public, max-age=3600, s-maxage=3600',
      },
    });
  } catch (error) {
    console.error('Error generating llms.txt:', error);
    return new NextResponse('Error generating documentation', { status: 500 });
  }
}

async function findMdxFiles(dir: string): Promise<string[]> {
  const files: string[] = [];

  async function scan(currentDir: string): Promise<void> {
    const entries = await readdir(currentDir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(currentDir, entry.name);

      if (entry.isDirectory()) {
        await scan(fullPath);
      } else if (entry.isFile() && entry.name.endsWith('.mdx')) {
        files.push(fullPath);
      }
    }
  }

  await scan(dir);
  return files;
}

function createTitleFromPath(filePath: string): string {
  // Remove .mdx extension and convert path to readable title
  const withoutExt = filePath.replace(/\.mdx$/, '');
  const segments = withoutExt.split('/');

  // Handle index files
  if (segments[segments.length - 1] === 'index') {
    segments.pop();
  }

  // Capitalize each segment and join
  return segments
    .map(segment => segment.replace(/-/g, ' ').replace(/\b\w/g, l => l.toUpperCase()))
    .join(' > ');
}