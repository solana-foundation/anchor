import type {
  FlatDoc,
  MetaFile,
  MetaItemOverride,
  SidebarBadge,
  SidebarGroup,
  SidebarLink,
  SidebarNode,
} from '@/types'
import { docHref, docLabel, getAllDocs, type Doc } from '@/lib/docs'
import { isCurrentPath, titleCase, trimTrailingSlash } from '@/lib/utils'

const metaModules = {
  ...(import.meta.glob('/src/content/docs/_meta.ts', {
    eager: true,
  }) as Record<string, { default: MetaFile }>),
  ...(import.meta.glob('/src/content/docs/**/_meta.ts', {
    eager: true,
  }) as Record<string, { default: MetaFile }>),
}

function metaFor(dirPath: string): MetaFile {
  const key = dirPath ? `/src/content/docs/${dirPath}/_meta.ts` : '/src/content/docs/_meta.ts'
  return metaModules[key]?.default ?? {}
}

type TreeNode = {
  name: string
  fullPath: string
  doc?: Doc
  children: TreeNode[]
}

function ensureNode(parent: TreeNode, name: string, fullPath: string): TreeNode {
  let existing = parent.children.find((c) => c.name === name)
  if (!existing) {
    existing = { name, fullPath, children: [] }
    parent.children.push(existing)
  }
  return existing
}

function buildTree(docs: Doc[]): TreeNode {
  const root: TreeNode = { name: '', fullPath: '', children: [] }

  for (const doc of docs) {
    const parts = doc.id.split('/')
    let node = root
    for (let i = 0; i < parts.length - 1; i++) {
      const dirPath = parts.slice(0, i + 1).join('/')
      node = ensureNode(node, parts[i], dirPath)
    }
    const lastName = parts[parts.length - 1]
    const leaf = ensureNode(node, lastName, doc.id)
    leaf.doc = doc
  }

  return root
}

type OrderKey = {
  order: number
  label: string
}

function compareByOrder(a: OrderKey, b: OrderKey): number {
  if (a.order !== b.order) return a.order - b.order
  return a.label.localeCompare(b.label)
}

function resolveDocLink(
  node: TreeNode,
  parentMeta: MetaFile,
  pathname: string,
): { link: SidebarLink; sortKey: OrderKey } | null {
  const doc = node.doc!
  if (doc.data.sidebar?.hidden) return null

  const override: MetaItemOverride = parentMeta.items?.[node.name] ?? {}
  if (override.hidden) return null

  const href = docHref(doc.id)
  const label = doc.data.sidebar?.label ?? override.label ?? docLabel(doc)
  const order = doc.data.sidebar?.order ?? override.order ?? Infinity
  const badge: SidebarBadge | undefined = doc.data.sidebar?.badge ?? override.badge

  return {
    link: {
      type: 'link',
      label,
      href,
      badge,
      isCurrent: isCurrentPath(href, pathname),
    },
    sortKey: { order, label },
  }
}

function resolveGroup(
  node: TreeNode,
  parentMeta: MetaFile,
  pathname: string,
): { group: SidebarGroup; sortKey: OrderKey } | null {
  const own = metaFor(node.fullPath)
  if (own.hidden) return null

  const override: MetaItemOverride = parentMeta.items?.[node.name] ?? {}
  if (override.hidden) return null

  const items = buildNodes(node, own, pathname)

  if (node.doc && !node.doc.data.sidebar?.hidden) {
    const indexOverride: MetaItemOverride = own.items?.index ?? {}
    if (!indexOverride.hidden) {
      const href = docHref(node.doc.id)
      const label = node.doc.data.sidebar?.label ?? indexOverride.label ?? 'Overview'
      const badge = node.doc.data.sidebar?.badge ?? indexOverride.badge
      items.unshift({
        type: 'link',
        label,
        href,
        badge,
        isCurrent: isCurrentPath(href, pathname),
      })
    }
  }

  if (items.length === 0) return null

  const label = own.label ?? override.label ?? titleCase(node.name)
  const order = override.order ?? own.order ?? Infinity
  const badge: SidebarBadge | undefined = override.badge ?? own.badge
  const forceOpen = override.forceOpen ?? own.forceOpen ?? false

  const hasActiveDescendant = items.some(
    (i) => (i.type === 'link' && i.isCurrent) || (i.type === 'group' && i.hasActiveDescendant),
  )

  return {
    group: {
      type: 'group',
      label,
      collapsed: forceOpen ? false : (own.collapsed ?? !hasActiveDescendant),
      forceOpen,
      badge,
      hasActiveDescendant,
      items,
    },
    sortKey: { order, label },
  }
}

function buildNodes(parent: TreeNode, parentMeta: MetaFile, pathname: string): SidebarNode[] {
  const resolved: Array<{ node: SidebarNode; sortKey: OrderKey }> = []

  for (const child of parent.children) {
    const isRootIndex = child.fullPath === 'index' && !!child.doc
    if (isRootIndex) continue

    if (child.children.length > 0) {
      const result = resolveGroup(child, parentMeta, pathname)
      if (result) resolved.push({ node: result.group, sortKey: result.sortKey })
    } else if (child.doc) {
      const result = resolveDocLink(child, parentMeta, pathname)
      if (result) resolved.push({ node: result.link, sortKey: result.sortKey })
    }
  }

  resolved.sort((a, b) => compareByOrder(a.sortKey, b.sortKey))
  return resolved.map((r) => r.node)
}

export async function getSidebarTree(pathname: string = '/'): Promise<SidebarNode[]> {
  const docs = await getAllDocs()
  const tree = buildTree(docs)
  const rootMeta = metaFor('')
  return buildNodes(tree, rootMeta, pathname)
}

function flattenTree(nodes: SidebarNode[], acc: FlatDoc[] = []): FlatDoc[] {
  for (const node of nodes) {
    if (node.type === 'link') {
      acc.push({
        id: '',
        href: node.href,
        title: node.label,
        label: node.label,
        hidden: false,
      })
    } else {
      flattenTree(node.items, acc)
    }
  }
  return acc
}

export async function getFlatDocOrder(): Promise<FlatDoc[]> {
  const tree = await getSidebarTree('/')
  return flattenTree(tree)
}

export type IndexChild = {
  label: string
  href: string
  description?: string
}

function findGroupChildren(nodes: SidebarNode[], href: string): SidebarNode[] | null {
  const normalized = trimTrailingSlash(href)
  for (const node of nodes) {
    if (node.type !== 'group') continue
    const hasIndex = node.items.some(
      (item) => item.type === 'link' && trimTrailingSlash(item.href) === normalized,
    )
    if (hasIndex) {
      return node.items.filter(
        (item) => !(item.type === 'link' && trimTrailingSlash(item.href) === normalized),
      )
    }
    const found = findGroupChildren(node.items, href)
    if (found !== null) return found
  }
  return null
}

export async function getIndexChildren(href: string): Promise<IndexChild[]> {
  const docs = await getAllDocs()
  const docMap = new Map(docs.map((d) => [docHref(d.id), d]))

  const tree = await getSidebarTree(href)
  const nodes = href === '/' ? tree : (findGroupChildren(tree, href) ?? [])

  return nodes.flatMap((node): IndexChild[] => {
    if (node.type === 'link') {
      const doc = docMap.get(node.href)
      return [{ label: node.label, href: node.href, description: doc?.data.description }]
    }
    const first = node.items[0]
    if (first?.type === 'link') {
      const doc = docMap.get(first.href)
      return [{ label: node.label, href: first.href, description: doc?.data.description }]
    }
    return []
  })
}

export async function getPrevNext(
  currentHref: string,
): Promise<{ prev: FlatDoc | null; next: FlatDoc | null }> {
  const flat = await getFlatDocOrder()
  const normalized = trimTrailingSlash(currentHref)
  const index = flat.findIndex((d) => trimTrailingSlash(d.href) === normalized)
  if (index === -1) {
    if (normalized === '/') {
      return { prev: null, next: flat[0] ?? null }
    }
    return { prev: null, next: null }
  }
  return {
    prev: index > 0 ? flat[index - 1] : null,
    next: index < flat.length - 1 ? flat[index + 1] : null,
  }
}
