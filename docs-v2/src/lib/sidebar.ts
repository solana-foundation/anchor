import type {
  FlatDoc,
  MetaFile,
  MetaItemOverride,
  SidebarBadge,
  SidebarGroup,
  SidebarLink,
  SidebarNode,
} from '@/types'
import { BASE_URL, docHref, docLabel, getAllDocs, type Doc } from '@/lib/docs'
import {
  candidateDocIdsForVersion,
  DOCS_VERSION_LABELS,
  type DocsVersion,
} from '@/lib/docs-versions'
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

type DocTreeNode = TreeNode & {
  doc: Doc
}

function hasDoc(node: TreeNode): node is DocTreeNode {
  return Boolean(node.doc)
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

export type SidebarContext = {
  docs: Doc[]
  tree: TreeNode
  docByHref: Map<string, Doc>
  rootMeta: MetaFile
}

export function createSidebarContext(docs: Doc[]): SidebarContext {
  return {
    docs,
    tree: buildTree(docs),
    docByHref: new Map(docs.map((doc) => [docHref(doc.id), doc])),
    rootMeta: metaFor(''),
  }
}

async function loadSidebarContext(): Promise<SidebarContext> {
  return createSidebarContext(await getAllDocs())
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
  node: DocTreeNode,
  parentMeta: MetaFile,
  pathname: string,
): { link: SidebarLink; sortKey: OrderKey } | null {
  const { doc } = node
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
      collapsed: forceOpen ? false : (override.collapsed ?? own.collapsed ?? !hasActiveDescendant),
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
    } else if (hasDoc(child)) {
      const result = resolveDocLink(child, parentMeta, pathname)
      if (result) resolved.push({ node: result.link, sortKey: result.sortKey })
    }
  }

  resolved.sort((a, b) => compareByOrder(a.sortKey, b.sortKey))
  return resolved.map((r) => r.node)
}

function urlForPath(pathname: string): URL {
  try {
    return new URL(pathname, 'https://anchor.local')
  } catch {
    return new URL('/', 'https://anchor.local')
  }
}

function pathnameWithinBase(pathname: string): string {
  let path = urlForPath(pathname).pathname

  if (BASE_URL !== '/' && path.startsWith(BASE_URL)) {
    path = path.slice(BASE_URL.length)
  } else {
    path = path.replace(/^\//, '')
  }

  return path.replace(/^\/+|\/+$/g, '')
}

function searchDocsVersion(pathname: string): DocsVersion | null {
  const version = urlForPath(pathname).searchParams.get('version')
  return version === 'v1' || version === 'v2' ? version : null
}

export function getActiveDocsVersion(pathname: string): DocsVersion | null {
  const [section] = pathnameWithinBase(pathname).split('/')
  return section === 'v1' || section === 'v2' ? section : null
}

function getFocusedDocsVersion(pathname: string): DocsVersion | null {
  const active = getActiveDocsVersion(pathname)
  if (active) return active

  const section = pathnameWithinBase(pathname).split('/')[0]
  if (section === 'updates') return searchDocsVersion(pathname) ?? 'v2'
  return null
}

function findChild(parent: TreeNode, name: string): TreeNode | undefined {
  return parent.children.find((child) => child.name === name)
}

function getVersionScopedNodes(
  tree: TreeNode,
  version: DocsVersion,
  rootMeta: MetaFile,
  pathname: string,
): SidebarNode[] {
  const versionNode = findChild(tree, version)
  const versionMeta = metaFor(version)
  const versionItems = versionNode ? buildNodes(versionNode, versionMeta, pathname) : []

  const updatesNode = findChild(tree, 'updates')
  const updatesGroup = updatesNode ? resolveGroup(updatesNode, rootMeta, pathname)?.group : null

  return updatesGroup ? [...versionItems, updatesGroup] : versionItems
}

export function getSidebarTreeFromContext(
  context: SidebarContext,
  pathname: string = '/',
): SidebarNode[] {
  const version = getFocusedDocsVersion(pathname)
  if (version) return getVersionScopedNodes(context.tree, version, context.rootMeta, pathname)
  return buildNodes(context.tree, context.rootMeta, pathname)
}

export async function getSidebarTree(pathname: string = '/'): Promise<SidebarNode[]> {
  return getSidebarTreeFromContext(await loadSidebarContext(), pathname)
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

export function getFlatDocOrderFromContext(
  context: SidebarContext,
  pathname: string = '/',
): FlatDoc[] {
  return flattenTree(getSidebarTreeFromContext(context, pathname))
}

export async function getFlatDocOrder(pathname: string = '/'): Promise<FlatDoc[]> {
  return getFlatDocOrderFromContext(await loadSidebarContext(), pathname)
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

export function getIndexChildrenFromContext(context: SidebarContext, href: string): IndexChild[] {
  const tree = getSidebarTreeFromContext(context, href)
  const nodes = href === BASE_URL ? tree : (findGroupChildren(tree, href) ?? [])

  return nodes.flatMap((node): IndexChild[] => {
    if (node.type === 'link') {
      const doc = context.docByHref.get(node.href)
      return [{ label: node.label, href: node.href, description: doc?.data.description }]
    }
    const first = node.items[0]
    if (first?.type === 'link') {
      const doc = context.docByHref.get(first.href)
      return [{ label: node.label, href: first.href, description: doc?.data.description }]
    }
    return []
  })
}

export async function getIndexChildren(href: string): Promise<IndexChild[]> {
  return getIndexChildrenFromContext(await loadSidebarContext(), href)
}

export function getPrevNextFromContext(
  context: SidebarContext,
  currentHref: string,
): { prev: FlatDoc | null; next: FlatDoc | null } {
  const flat = getFlatDocOrderFromContext(context, currentHref)
  const normalized = trimTrailingSlash(currentHref)
  const index = flat.findIndex((d) => trimTrailingSlash(d.href) === normalized)
  if (index === -1) {
    if (normalized === trimTrailingSlash(BASE_URL)) {
      return { prev: null, next: flat[0] ?? null }
    }
    return { prev: null, next: null }
  }
  return {
    prev: index > 0 ? flat[index - 1] : null,
    next: index < flat.length - 1 ? flat[index + 1] : null,
  }
}

export async function getPrevNext(
  currentHref: string,
): Promise<{ prev: FlatDoc | null; next: FlatDoc | null }> {
  return getPrevNextFromContext(await loadSidebarContext(), currentHref)
}

export type DocsVersionSwitchOption = {
  version: DocsVersion
  label: string
  href: string
  active: boolean
  badge?: SidebarBadge
}

function fallbackVersionHref(version: DocsVersion): string {
  return docHref(`${version}/index`)
}

function matchingVersionHref(docs: Doc[], version: DocsVersion, relativePath: string): string {
  const ids = new Set(docs.map((doc) => doc.id))
  const candidates = candidateDocIdsForVersion(version, relativePath)
  const match = candidates.find((id) => ids.has(id))
  return match ? docHref(match) : fallbackVersionHref(version)
}

function hrefWithVersionSearch(pathname: string, version: DocsVersion): string {
  const url = urlForPath(pathname)
  url.searchParams.set('version', version)
  return `${url.pathname}${url.search}`
}

export function getDocsVersionSwitchOptionsFromContext(
  context: SidebarContext,
  pathname: string,
): DocsVersionSwitchOption[] {
  const relative = pathnameWithinBase(pathname)
  const parts = relative.split('/').filter(Boolean)
  const active = getFocusedDocsVersion(pathname)
  const isUpdatesPath = parts[0] === 'updates'
  const sameVersionPath = parts[0] === 'v1' || parts[0] === 'v2' ? parts.slice(1).join('/') : ''

  return [
    {
      version: 'v2',
      label: DOCS_VERSION_LABELS.v2,
      href: isUpdatesPath
        ? hrefWithVersionSearch(pathname, 'v2')
        : matchingVersionHref(context.docs, 'v2', sameVersionPath),
      active: active === 'v2',
    },
    {
      version: 'v1',
      label: DOCS_VERSION_LABELS.v1,
      href: isUpdatesPath
        ? hrefWithVersionSearch(pathname, 'v1')
        : matchingVersionHref(context.docs, 'v1', sameVersionPath),
      active: active === 'v1',
    },
  ]
}

export async function getDocsVersionSwitchOptions(
  pathname: string,
): Promise<DocsVersionSwitchOption[]> {
  return getDocsVersionSwitchOptionsFromContext(await loadSidebarContext(), pathname)
}
