import { isElementHidden, trimTrailingSlash, withDatasetFlag } from './dom'
import { readJsonRecord, readNumber, removeStorage, writeJson, writeStorage } from './storage'

const SIDEBAR_GROUP_STATE_KEY = 'anchor-docs-sidebar-groups'
const SIDEBAR_SCROLL_STATE_KEY = 'anchor-docs-sidebar-scroll'
const SIDEBAR_PENDING_SCROLL_KEY = 'anchor-docs-sidebar-pending-scroll'
const SIDEBAR_VIEWPORT_SELECTOR =
  'nav[aria-label="Documentation"] [data-slot="scroll-area-viewport"]'

let lifecycleReady = false
let syncingSidebarGroups = false

const sidebarGroupsWithPersistence = new WeakSet<HTMLDetailsElement>()
const sidebarViewportsWithPersistence = new WeakSet<HTMLElement>()

function readSidebarGroupState(): Record<string, boolean> {
  return readJsonRecord<boolean>(localStorage, SIDEBAR_GROUP_STATE_KEY)
}

function writeSidebarGroupState(state: Record<string, boolean>): void {
  writeJson(localStorage, SIDEBAR_GROUP_STATE_KEY, state)
}

function readSidebarScrollState(): Record<string, number> {
  return readJsonRecord<number>(localStorage, SIDEBAR_SCROLL_STATE_KEY)
}

function writeSidebarScrollState(state: Record<string, number>): void {
  writeJson(localStorage, SIDEBAR_SCROLL_STATE_KEY, state)
}

function sidebarGroupKey(group: HTMLElement): string | null {
  const key = group.dataset.groupPath
  return key && key.length > 0 ? key : null
}

function matchingSidebarGroups(key: string): NodeListOf<HTMLDetailsElement> {
  return document.querySelectorAll<HTMLDetailsElement>(
    `details[data-sidebar-group][data-group-path="${CSS.escape(key)}"]`,
  )
}

function setMatchingSidebarGroups(key: string, open: boolean, source: HTMLDetailsElement): void {
  syncingSidebarGroups = true

  matchingSidebarGroups(key).forEach((group) => {
    if (group !== source) group.open = open
  })

  syncingSidebarGroups = false
}

function restoreSidebarGroups(): void {
  const state = readSidebarGroupState()

  document
    .querySelectorAll<HTMLDetailsElement>('details[data-sidebar-group][data-group-path]')
    .forEach((group) => {
      const key = sidebarGroupKey(group)
      if (!key || state[key] === undefined) return
      group.open = state[key]
    })
}

function setupPersistentSidebarGroups(): void {
  restoreSidebarGroups()

  document
    .querySelectorAll<HTMLDetailsElement>('details[data-sidebar-group][data-group-path]')
    .forEach((group) => {
      if (sidebarGroupsWithPersistence.has(group)) return
      sidebarGroupsWithPersistence.add(group)

      group.addEventListener('toggle', () => {
        if (syncingSidebarGroups) return

        const key = sidebarGroupKey(group)
        if (!key) return

        const state = readSidebarGroupState()
        state[key] = group.open
        writeSidebarGroupState(state)
        setMatchingSidebarGroups(key, group.open, group)
      })
    })
}

function sidebarViewports(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>(SIDEBAR_VIEWPORT_SELECTOR))
}

function sidebarScrollKey(viewport: HTMLElement): string {
  const version =
    viewport.closest<HTMLElement>('[data-version-sidebar]')?.dataset.versionSidebar ?? 'default'
  const placement = viewport.closest('#mobile-sidebar') ? 'mobile' : 'desktop'
  return `${placement}:${version}`
}

function saveSidebarViewportScroll(viewport: HTMLElement): void {
  const state = readSidebarScrollState()
  state[sidebarScrollKey(viewport)] = viewport.scrollTop
  writeSidebarScrollState(state)
}

export function saveVisibleSidebarScrolls(): void {
  sidebarViewports().forEach((viewport) => {
    if (!isElementHidden(viewport)) saveSidebarViewportScroll(viewport)
  })
}

export function savePendingSidebarScroll(): void {
  const viewport = sidebarViewports().find((candidate) => !isElementHidden(candidate))
  if (!viewport) return

  writeStorage(sessionStorage, SIDEBAR_PENDING_SCROLL_KEY, String(viewport.scrollTop))
}

export function restoreSidebarScrolls(): void {
  const pendingScrollTop = readNumber(sessionStorage, SIDEBAR_PENDING_SCROLL_KEY)
  const state = readSidebarScrollState()
  let usedPendingScroll = false

  sidebarViewports().forEach((viewport) => {
    if (isElementHidden(viewport)) return

    const scrollTop = pendingScrollTop ?? state[sidebarScrollKey(viewport)]
    if (typeof scrollTop !== 'number') return

    viewport.scrollTop = Math.max(
      0,
      Math.min(scrollTop, viewport.scrollHeight - viewport.clientHeight),
    )
    if (pendingScrollTop !== null) usedPendingScroll = true
  })

  if (usedPendingScroll) removeStorage(sessionStorage, SIDEBAR_PENDING_SCROLL_KEY)
}

function setupPersistentSidebarScrolls(): void {
  restoreSidebarScrolls()

  sidebarViewports().forEach((viewport) => {
    if (sidebarViewportsWithPersistence.has(viewport)) return
    sidebarViewportsWithPersistence.add(viewport)

    viewport.addEventListener('scroll', () => saveSidebarViewportScroll(viewport), {
      passive: true,
    })
  })
}

function expandActiveLinkParents(link: HTMLElement): void {
  let node: HTMLElement | null = link.parentElement

  while (node) {
    if (node instanceof HTMLDetailsElement) node.open = true
    node = node.parentElement
  }
}

function updateActiveSidebarLink(): void {
  const current = trimTrailingSlash(window.location.pathname)

  document.querySelectorAll<HTMLAnchorElement>('[data-sidebar-link]').forEach((link) => {
    const href = link.getAttribute('data-sidebar-link') ?? ''
    const isActive = trimTrailingSlash(href) === current

    if (isActive) {
      link.setAttribute('aria-current', 'page')
      expandActiveLinkParents(link)
    } else {
      link.removeAttribute('aria-current')
    }
  })
}

function setupSidebar(): void {
  withDatasetFlag('[data-sidebar-root]', 'sidebarSettling', () => {
    setupPersistentSidebarGroups()
    updateActiveSidebarLink()
    setupPersistentSidebarScrolls()
  })
}

export function mountDocsSidebar(): void {
  setupSidebar()

  if (lifecycleReady) return
  lifecycleReady = true

  document.addEventListener('astro:before-swap', saveVisibleSidebarScrolls)
  document.addEventListener('astro:after-swap', setupSidebar)
  window.addEventListener('anchor-docs:version-focus', restoreSidebarScrolls)
}
