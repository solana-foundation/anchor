import type { Element, ElementContent, Root } from 'hast'

export type HastNode = Root | ElementContent
export type HastParent = Root | Element
export type VisitAction = 'skip' | void

export function isElement(node: HastNode): node is Element {
  return node.type === 'element'
}

export function getElementClasses(node: Element): string[] {
  const value = node.properties?.className ?? node.properties?.class

  if (Array.isArray(value)) return value.map(String)
  if (typeof value === 'string') return value.split(/\s+/).filter(Boolean)

  return []
}

export function hasClass(node: Element, className: string): boolean {
  return getElementClasses(node).includes(className)
}

export function getNodeText(node: ElementContent): string {
  if (node.type === 'text') return node.value
  if (!isElement(node)) return ''

  return node.children.map(getNodeText).join('')
}

export function findElementChild(node: Element, tagName: string): Element | null {
  return (
    node.children.find(
      (child): child is Element => isElement(child) && child.tagName === tagName,
    ) ?? null
  )
}

export function replaceChild(parent: HastParent, index: number, child: ElementContent): void {
  const children = parent.children as ElementContent[]
  children[index] = child
}

function childNodes(node: HastNode): ElementContent[] | null {
  if (node.type === 'root') return node.children as ElementContent[]
  if (isElement(node)) return node.children

  return null
}

export function visitHast(
  tree: Root,
  visitor: (node: HastNode, parent: HastParent | null, index: number | null) => VisitAction,
): void {
  const walk = (node: HastNode, parent: HastParent | null, index: number | null): void => {
    if (visitor(node, parent, index) === 'skip') return

    const children = childNodes(node)
    if (!children) return

    for (let i = 0; i < children.length; i++) {
      walk(children[i], node as HastParent, i)
    }
  }

  walk(tree, null, null)
}
