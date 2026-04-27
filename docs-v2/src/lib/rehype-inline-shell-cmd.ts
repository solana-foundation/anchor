import type { Element, ElementContent, Root, Text } from 'hast'

const PREFIX = '$ '

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

function firstTextNode(node: Element): Text | null {
  for (const child of node.children) {
    if (child.type === 'text') return child
    if (child.type === 'element') {
      const found = firstTextNode(child)
      if (found) return found
    }
  }
  return null
}

function findCodeChild(node: Element): Element | null {
  for (const child of node.children) {
    if (child.type === 'element' && child.tagName === 'code') return child
  }
  return null
}

function makeButton(codeNode: Element, cmd: string): Element {
  return {
    type: 'element',
    tagName: 'button',
    properties: {
      type: 'button',
      className: ['inline-shell-cmd'],
      'data-copy': cmd,
      'aria-label': `Copy command: ${cmd}`,
      title: `Copy: ${cmd}`,
    },
    children: [
      {
        type: 'element',
        tagName: 'span',
        properties: { className: ['shell-prompt'], 'aria-hidden': 'true' },
        children: [{ type: 'text', value: '$' }],
      },
      codeNode,
    ],
  }
}

export function rehypeInlineShellCmd() {
  return (tree: Root) => {
    const visit = (node: any, parent: any, index: number | null) => {
      if (node.type !== 'element') {
        if (node.children) {
          for (let i = 0; i < node.children.length; i++) {
            visit(node.children[i], node, i)
          }
        }
        return
      }

      // Block code: skip the whole subtree (handled by ec-shell-prompt).
      if (node.tagName === 'pre') return
      // Idempotency: don't re-wrap.
      if (hasClass(node, 'inline-shell-cmd')) return

      // Case A: shiki-wrapped inline code (`{:bash}`, `{:ansi}`, etc.)
      // Structure: <span class="shiki ..."><code>…$ cmd…</code></span>
      // Replace the .shiki wrapper entirely so it doesn't get its own pill styling.
      if (
        node.tagName === 'span' &&
        hasClass(node, 'shiki') &&
        parent &&
        index !== null
      ) {
        const codeChild = findCodeChild(node)
        if (codeChild) {
          const first = firstTextNode(codeChild)
          if (first && first.value.startsWith(PREFIX)) {
            const cmd = getText(codeChild).slice(PREFIX.length)
            if (cmd.length > 0) {
              first.value = first.value.slice(PREFIX.length)
              parent.children[index] = makeButton(codeChild, cmd)
              return
            }
          }
        }
      }

      // Case B: plain inline <code> with no language tag.
      if (node.tagName === 'code' && parent && index !== null) {
        const first = firstTextNode(node)
        if (first && first.value.startsWith(PREFIX)) {
          const fullText = getText(node)
          const cmd = fullText.slice(PREFIX.length)
          if (cmd.length > 0) {
            first.value = first.value.slice(PREFIX.length)
            parent.children[index] = makeButton(node, cmd)
            return
          }
        }
      }

      if (node.children) {
        for (let i = 0; i < node.children.length; i++) {
          visit(node.children[i], node, i)
        }
      }
    }
    visit(tree as any, null, null)
  }
}
