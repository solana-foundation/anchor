import { compiler } from 'markdown-to-jsx';

export async function extractTextFromMDX(mdxContent: string): Promise<string> {
  try {
    // Use markdown-to-jsx compiler to parse MDX and extract text
    const compiled = compiler(mdxContent, {
      // Override components to extract text content
      overrides: {
        // Handle custom fumadocs components
        Cards: ({ children }) => extractChildrenText(children),
        Card: ({ title, children, href }) => {
          let text = '';
          if (title) text += `${title}: `;
          text += extractChildrenText(children);
          if (href) text += ` (Link: ${href})`;
          return text + '\n';
        },
        // Handle common HTML elements
        h1: ({ children }) => `# ${extractChildrenText(children)}\n\n`,
        h2: ({ children }) => `## ${extractChildrenText(children)}\n\n`,
        h3: ({ children }) => `### ${extractChildrenText(children)}\n\n`,
        h4: ({ children }) => `#### ${extractChildrenText(children)}\n\n`,
        h5: ({ children }) => `##### ${extractChildrenText(children)}\n\n`,
        h6: ({ children }) => `###### ${extractChildrenText(children)}\n\n`,
        p: ({ children }) => `${extractChildrenText(children)}\n\n`,
        ul: ({ children }) => `${extractChildrenText(children)}\n`,
        ol: ({ children }) => `${extractChildrenText(children)}\n`,
        li: ({ children }) => `• ${extractChildrenText(children)}\n`,
        strong: ({ children }) => `**${extractChildrenText(children)}**`,
        em: ({ children }) => `*${extractChildrenText(children)}*`,
        a: ({ href, children }) => `${extractChildrenText(children)} (${href})`,
        code: ({ children }) => `\`${extractChildrenText(children)}\``,
        pre: ({ children }) => `\n${extractChildrenText(children)}\n`,
        blockquote: ({ children }) => `> ${extractChildrenText(children)}\n\n`,
        // Remove other JSX components but keep their text content
        Callout: ({ children }) => extractChildrenText(children),
        Note: ({ children }) => extractChildrenText(children),
        Tip: ({ children }) => extractChildrenText(children),
        Warning: ({ children }) => extractChildrenText(children),
        // Default handler for unknown components
        default: ({ children }) => extractChildrenText(children),
      },
    });

    return typeof compiled === 'string' ? compiled : extractChildrenText(compiled);
  } catch (error) {
    console.warn('Error extracting text from MDX:', error);
    // Fallback: try to extract text using regex
    return extractTextFallback(mdxContent);
  }
}

function extractChildrenText(children: any): string {
  if (!children) return '';

  if (typeof children === 'string') return children;
  if (typeof children === 'number') return children.toString();

  if (Array.isArray(children)) {
    return children.map(extractChildrenText).join('');
  }

  if (typeof children === 'object' && children.props && children.props.children) {
    return extractChildrenText(children.props.children);
  }

  return '';
}

function extractTextFallback(content: string): string {
  // Remove frontmatter
  let text = content.replace(/^---[\s\S]*?---\n/, '');

  // Remove import statements
  text = text.replace(/^import.*from.*;\n/gm, '');

  // Remove JSX component tags but keep content
  text = text.replace(/<[^>]+>/g, '');

  // Clean up extra whitespace
  text = text.replace(/\n{3,}/g, '\n\n');

  return text.trim();
}