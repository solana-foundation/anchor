import type { Element, Root } from 'hast'
import { hasClass, isElement, replaceChild, visitHast } from './hast-utils'

function tableWrapper(node: Element): Element {
  return {
    type: 'element',
    tagName: 'div',
    properties: { className: ['table-wrapper'] },
    children: [node],
  }
}

export function rehypeWrapTables() {
  return (tree: Root) => {
    visitHast(tree, (node, parent, index) => {
      if (!isElement(node)) return
      if (node.tagName !== 'table' || !parent || index === null) return
      if (isElement(parent) && parent.tagName === 'div' && hasClass(parent, 'table-wrapper')) return

      replaceChild(parent, index, tableWrapper(node))
      return 'skip'
    })
  }
}
