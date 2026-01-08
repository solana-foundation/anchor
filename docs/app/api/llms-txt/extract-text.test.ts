import { extractTextFromMDX } from './ast-extract';

describe('extractTextFromMDX', () => {
  it('should extract text from simple markdown', async () => {
    const mdx = `
# Hello World

This is a paragraph with **bold** and *italic* text.

## Code Example

\`\`\`rust
fn main() {
    println!("Hello, Anchor!");
}
\`\`\`

> This is a blockquote
`;

    const result = await extractTextFromMDX(mdx);
    expect(result).toContain('Hello World');
    expect(result).toContain('This is a paragraph');
    expect(result).toContain('println!("Hello, Anchor!")');
    expect(result).toContain('This is a blockquote');
  });

  it('should handle fumadocs Cards component', async () => {
    const mdx = `
<Cards>
<Card title="Installation" href="/docs/installation">
Step-by-step guide to install Anchor framework.
</Card>
</Cards>
`;

    const result = await extractTextFromMDX(mdx);
    expect(result).toContain('Installation');
    expect(result).toContain('Step-by-step guide');
    expect(result).toContain('/docs/installation');
  });

  it('should preserve code blocks', async () => {
    const mdx = `
\`\`\`toml
[dependencies]
anchor-lang = "0.30.0"
\`\`\`
`;

    const result = await extractTextFromMDX(mdx);
    expect(result).toContain('[dependencies]');
    expect(result).toContain('anchor-lang = "0.30.0"');
  });

  it('should handle frontmatter', async () => {
    const mdx = `---
title: Test Page
description: A test page
---

# Content

This is the content.
`;

    const result = await extractTextFromMDX(mdx);
    expect(result).toContain('Content');
    expect(result).toContain('This is the content');
    expect(result).not.toContain('title: Test Page');
  });

  it('should handle complex nested components', async () => {
    const mdx = `
import { Download } from "lucide-react";

<Card icon={<Download className="text-purple-300" />} title='Installation' href='/docs/installation'>
Step-by-step guide content here.
</Card>
`;

    const result = await extractTextFromMDX(mdx);
    expect(result).toContain('Installation');
    expect(result).toContain('Step-by-step guide content here');
  });
});