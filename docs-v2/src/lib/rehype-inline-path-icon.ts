import type { Element, ElementContent, Root } from 'hast'

// Lucide icon paths (viewBox 0 0 24 24, stroke currentColor).
const FILE_PATHS = [
  'M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z',
  'M14 2v4a2 2 0 0 0 2 2h4',
]
const FOLDER_PATHS = [
  'M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z',
]

const EXTENSIONS = [
  'toml', 'rs', 'ts', 'tsx', 'js', 'jsx', 'mjs', 'cjs',
  'json', 'jsonc', 'json5', 'md', 'mdx', 'yaml', 'yml',
  'lock', 'txt', 'sh', 'bash', 'zsh', 'fish', 'css',
  'scss', 'html', 'svg', 'png', 'jpg', 'jpeg', 'webp',
  'gif', 'env', 'sol', 'graphql', 'gql', 'sql', 'xml',
  'astro', 'vue', 'svelte', 'ini', 'cfg',
]
const EXT_RE = new RegExp(`\\.(${EXTENSIONS.join('|')})$`, 'i')
const DOTFILE_RE =
  /^\.(gitignore|env|prettierrc|eslintrc|npmrc|yarnrc|nvmrc|editorconfig|gitkeep|gitattributes)/i

function hasClass(node: Element, className: string): boolean {
  const props = node.properties
  if (!props) return false
  const c = (props as any).className ?? (props as any).class
  if (Array.isArray(c)) return c.includes(className)
  if (typeof c === 'string') return c.split(/\s+/).includes(className)
  return false
}

function getText(node: ElementContent): string {
  if (node.type === 'text') return node.value
  if (node.type !== 'element' || !node.children) return ''
  return node.children.map(getText).join('')
}

function classify(text: string): 'file' | 'folder' | null {
  if (!text || text.length > 200) return null
  if (/\s/.test(text)) return null
  if (text.includes('://')) return null
  if (text.startsWith('@')) return null
  if (text.startsWith('-')) return null
  if (text.startsWith('$ ')) return null
  // A single token like "x" or "foo" without a real signal isn't a path.
  if (text.endsWith('/')) return 'folder'
  if (text.includes('/')) return 'file'
  if (EXT_RE.test(text)) return 'file'
  if (DOTFILE_RE.test(text)) return 'file'
  return null
}

function svgIcon(paths: string[], kind: 'file' | 'folder'): Element {
  return {
    type: 'element',
    tagName: 'svg',
    properties: {
      viewBox: '0 0 24 24',
      fill: 'none',
      stroke: 'currentColor',
      strokeWidth: 2,
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
      'aria-hidden': 'true',
      className: ['inline-path-icon', `is-${kind}`],
    },
    children: paths.map((d) => ({
      type: 'element',
      tagName: 'path',
      properties: { d },
      children: [],
    })),
  }
}

function alreadyDecorated(node: Element): boolean {
  const first = node.children?.[0]
  return (
    first?.type === 'element' &&
    first.tagName === 'svg' &&
    hasClass(first, 'inline-path-icon')
  )
}

function decorate(codeNode: Element, kind: 'file' | 'folder'): void {
  if (alreadyDecorated(codeNode)) return
  const icon = svgIcon(kind === 'folder' ? FOLDER_PATHS : FILE_PATHS, kind)
  codeNode.children.unshift(icon)
}

function findCodeChild(node: Element): Element | null {
  for (const child of node.children) {
    if (child.type === 'element' && child.tagName === 'code') return child
  }
  return null
}

export function rehypeInlinePathIcon() {
  return (tree: Root) => {
    const visit = (node: any) => {
      if (node.type !== 'element') {
        if (node.children) for (const c of node.children) visit(c)
        return
      }

      // Skip block code and our own command pills.
      if (node.tagName === 'pre') return
      if (hasClass(node, 'inline-shell-cmd')) return

      // Shiki-wrapped inline code: decorate the inner <code>, don't recurse in.
      if (node.tagName === 'span' && hasClass(node, 'shiki')) {
        const codeChild = findCodeChild(node)
        if (codeChild) {
          const kind = classify(getText(codeChild))
          if (kind) decorate(codeChild, kind)
        }
        return
      }

      // Plain inline <code>.
      if (node.tagName === 'code') {
        const kind = classify(getText(node))
        if (kind) decorate(node, kind)
        return
      }

      if (node.children) for (const c of node.children) visit(c)
    }
    visit(tree as any)
  }
}
