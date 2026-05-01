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
import { isCurrentPath, titleCase, trimTrailingSlash } from '@/lib/utils'

export type DocsVersion = 'v1' | 'v2'

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

export async function getSidebarTree(pathname: string = '/'): Promise<SidebarNode[]> {
  const docs = await getAllDocs()
  const tree = buildTree(docs)
  const rootMeta = metaFor('')
  const version = getFocusedDocsVersion(pathname)
  if (version) return getVersionScopedNodes(tree, version, rootMeta, pathname)
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

export async function getFlatDocOrder(pathname: string = '/'): Promise<FlatDoc[]> {
  const tree = await getSidebarTree(pathname)
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
  const nodes = href === BASE_URL ? tree : (findGroupChildren(tree, href) ?? [])

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
  const flat = await getFlatDocOrder(currentHref)
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

export type DocsVersionSwitchOption = {
  version: DocsVersion
  label: string
  href: string
  active: boolean
  badge?: SidebarBadge
}

function idFromVersionPath(version: DocsVersion, relativePath: string): string {
  return relativePath ? `${version}/${relativePath}` : `${version}/index`
}

function fallbackVersionHref(version: DocsVersion): string {
  return docHref(`${version}/index`)
}

const VERSION_ROUTE_ALIASES: Record<DocsVersion, Record<string, string>> = {
  v1: {
    'get-started/first-program': 'get-started/local-development',
    'get-started/migrating-from-v1': 'get-started/local-development',
    'fundamentals/accounts-and-context': 'programs/account-types',
    'fundamentals/account-validation': 'reference/account-constraints',
    'fundamentals/pdas-and-resolution': 'fundamentals/pdas',
    'programs/account-data-model': 'programs/account-types',
    'programs/pod-types': 'programs/zero-copy',
    'programs/borsh-accounts-and-realloc': 'programs/account-space-and-realloc',
    'programs/errors-and-require': 'programs/errors',
    'programs/extensibility': 'programs/account-types',
    'reference/macros-and-attributes': 'reference/account-constraints',
    'reference/account-types': 'programs/account-types',
    'reference/feature-flags': 'reference/anchor-toml',
    'reference/examples-and-benchmarks': 'reference/examples',
    'reference/alpha-limitations': 'reference/index',
    'security/secure-by-default': 'security/footguns',
    'security/production-builds': 'security/verifiable-builds',
    'security/performance-and-optimizations': 'security/verifiable-builds',
    'testing/profiling-and-debugger': 'testing/index',
    'testing/coverage': 'testing/index',
  },
  v2: {
    'get-started/solana-playground': 'get-started/first-program',
    'get-started/local-development': 'get-started/first-program',
    'fundamentals/pdas': 'fundamentals/pdas-and-resolution',
    'programs/account-space-and-realloc': 'programs/borsh-accounts-and-realloc',
    'programs/errors': 'programs/errors-and-require',
    'programs/zero-copy': 'programs/account-data-model',
    'reference/avm': 'reference/cli',
    'reference/rust-to-js-types': 'clients/typescript',
    'reference/examples': 'reference/examples-and-benchmarks',
    'security/sealevel-attacks': 'security/secure-by-default',
    'security/footguns': 'security/secure-by-default',
    'security/verifiable-builds': 'security/production-builds',
    'testing/mollusk': 'testing/litesvm',
  },
}

function sectionFallback(version: DocsVersion, relativePath: string): string {
  const [section] = relativePath.split('/')
  if (!section) return ''
  if (section === 'get-started') {
    return version === 'v2' ? 'get-started/first-program' : 'get-started/local-development'
  }
  return `${section}/index`
}

function versionPathCandidates(version: DocsVersion, relativePath: string): string[] {
  const alias = VERSION_ROUTE_ALIASES[version][relativePath]
  const fallback = sectionFallback(version, relativePath)
  return [relativePath, alias, fallback].filter(
    (candidate): candidate is string => typeof candidate === 'string' && candidate.length > 0,
  )
}

function versionDocIdCandidates(version: DocsVersion, relativePath: string): string[] {
  const baseId = idFromVersionPath(version, relativePath)
  if (!relativePath) return [baseId]

  const ids = [baseId, `${baseId}/index`]
  if (relativePath.endsWith('/index')) {
    ids.push(idFromVersionPath(version, relativePath.slice(0, -'/index'.length)))
  }
  return ids
}

function matchingVersionHref(docs: Doc[], version: DocsVersion, relativePath: string): string {
  const ids = new Set(docs.map((doc) => doc.id))
  const candidates = versionPathCandidates(version, relativePath).flatMap((path) =>
    versionDocIdCandidates(version, path),
  )
  const match = candidates.find((id) => ids.has(id))
  return match ? docHref(match) : fallbackVersionHref(version)
}

function hrefWithVersionSearch(pathname: string, version: DocsVersion): string {
  const url = urlForPath(pathname)
  url.searchParams.set('version', version)
  return `${url.pathname}${url.search}`
}

export async function getDocsVersionSwitchOptions(
  pathname: string,
): Promise<DocsVersionSwitchOption[]> {
  const docs = await getAllDocs()
  const relative = pathnameWithinBase(pathname)
  const parts = relative.split('/').filter(Boolean)
  const active = getFocusedDocsVersion(pathname) ?? 'v2'
  const isUpdatesPath = parts[0] === 'updates'
  const sameVersionPath = parts[0] === 'v1' || parts[0] === 'v2' ? parts.slice(1).join('/') : ''

  return [
    {
      version: 'v2',
      label: '2.0 alpha',
      href: isUpdatesPath
        ? hrefWithVersionSearch(pathname, 'v2')
        : matchingVersionHref(docs, 'v2', sameVersionPath),
      active: active === 'v2',
    },
    {
      version: 'v1',
      label: '1.0.1',
      href: isUpdatesPath
        ? hrefWithVersionSearch(pathname, 'v1')
        : matchingVersionHref(docs, 'v1', sameVersionPath),
      active: active === 'v1',
    },
  ]
}
