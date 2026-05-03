import type { Element, Properties, Root, Text } from 'hast'
import {
  findElementChild,
  getNodeText,
  hasClass,
  isElement,
  replaceChild,
  visitHast,
} from './hast-utils'

const PREFIX = '$ '

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

// Pills longer than this can overflow narrow contexts (table cells, TOC),
// since shell command text often has no break opportunities (commit hashes,
// flag values). Mark them so CSS can opt them into `overflow-wrap: anywhere`.
const LONG_PILL = 24

function makeButton(codeNode: Element, cmd: string): Element {
  // `<span role="button">`, not `<button>`. `<button>` is a form control,
  // and even with `display: inline` it preserves its bounding box across
  // line wraps — `box-decoration-break: clone` therefore can't clone the
  // pill chrome onto each line fragment, leaving long pills as one giant
  // rectangle. A `<span>` is naturally inline and clones cleanly.
  const properties: Properties = {
    role: 'button',
    tabIndex: 0,
    className: ['inline-shell-cmd'],
    'data-copy': cmd,
    'aria-label': `Copy command: ${cmd}`,
    title: `Copy: ${cmd}`,
  }
  if (cmd.length > LONG_PILL) properties['data-long-pill'] = 'true'
  return {
    type: 'element',
    tagName: 'span',
    properties,
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
    visitHast(tree, (node, parent, index) => {
      if (!isElement(node)) return

      // Block code: skip the whole subtree (handled by ec-shell-prompt).
      if (node.tagName === 'pre') return 'skip'
      // Idempotency: don't re-wrap.
      if (hasClass(node, 'inline-shell-cmd')) return 'skip'

      // Case A: shiki-wrapped inline code (`{:bash}`, `{:ansi}`, etc.)
      // Structure: <span class="shiki ..."><code>…$ cmd…</code></span>
      // Replace the .shiki wrapper entirely so it doesn't get its own pill styling.
      if (node.tagName === 'span' && hasClass(node, 'shiki') && parent && index !== null) {
        const codeChild = findElementChild(node, 'code')
        if (codeChild) {
          const first = firstTextNode(codeChild)
          if (first && first.value.startsWith(PREFIX)) {
            const cmd = getNodeText(codeChild).slice(PREFIX.length)
            if (cmd.length > 0) {
              first.value = first.value.slice(PREFIX.length)
              replaceChild(parent, index, makeButton(codeChild, cmd))
              return 'skip'
            }
          }
        }
      }

      // Case B: plain inline <code> with no language tag.
      if (node.tagName === 'code' && parent && index !== null) {
        const first = firstTextNode(node)
        if (first && first.value.startsWith(PREFIX)) {
          const fullText = getNodeText(node)
          const cmd = fullText.slice(PREFIX.length)
          if (cmd.length > 0) {
            first.value = first.value.slice(PREFIX.length)
            replaceChild(parent, index, makeButton(node, cmd))
            return 'skip'
          }
        }
      }
    })
  }
}
