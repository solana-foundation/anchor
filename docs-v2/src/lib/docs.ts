import { DOCS } from '@/consts'
import { getCollection, type CollectionEntry } from 'astro:content'

export type Doc = CollectionEntry<'docs'>

export async function getAllDocs(): Promise<Doc[]> {
  const docs = await getCollection('docs')
  return docs.filter((doc) => !doc.data.draft)
}

export function docHref(id: string): string {
  if (id === 'index') return '/'
  if (id.endsWith('/index')) return '/' + id.slice(0, -'/index'.length) + '/'
  return '/' + id + '/'
}

export function docSlugFromId(id: string): string | undefined {
  if (id === 'index') return undefined
  if (id.endsWith('/index')) return id.slice(0, -'/index'.length)
  return id
}

export function docLabel(doc: Doc): string {
  return doc.data.sidebar?.label ?? doc.data.title
}

export function getEditUrl(doc: Doc): string | null {
  if (doc.data.editUrl === false) return null
  if (typeof doc.data.editUrl === 'string') return doc.data.editUrl
  if (!DOCS.defaultEditUrl) return null
  if (!DOCS.editUrlBase) return null
  const filePath = doc.filePath
  if (!filePath) return null
  const rel = filePath.replace(/^.*\/src\/content\/docs\//, '')
  return DOCS.editUrlBase.replace(/\/+$/, '') + '/' + rel
}

export function resolveLastUpdated(doc: Doc): Date | null {
  const value = doc.data.lastUpdated
  if (value === false) return null
  if (value instanceof Date) return value
  if (!DOCS.defaultLastUpdated && value !== true) return null
  const injected = (doc.data as unknown as { _lastUpdated?: Date })._lastUpdated
  return injected ?? null
}

export function resolveTOC(doc: Doc): {
  enabled: boolean
  minDepth: number
  maxDepth: number
} {
  const cfg = doc.data.tableOfContents
  const defaults = DOCS.defaultTableOfContents
  if (cfg === false) return { enabled: false, ...defaults }
  if (cfg === true || cfg === undefined) return { enabled: true, ...defaults }
  return {
    enabled: true,
    minDepth: cfg.minDepth ?? defaults.minDepth,
    maxDepth: cfg.maxDepth ?? defaults.maxDepth,
  }
}
