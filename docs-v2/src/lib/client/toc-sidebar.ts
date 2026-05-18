import { findScrollAreaViewport, withDatasetFlag } from './dom'
import {
  buildHeadingRegions,
  centerElementInScrollContainer,
  getContentHeadings,
  getVisibleHeadingIds,
  headingHtml,
  sameStringArray,
  setVerticalScrollMask,
  type HeadingRegion,
} from './toc'

type TocSidebarState = {
  links: HTMLElement[]
  activeIds: string[]
  headings: HTMLElement[]
  regions: HeadingRegion[]
  scrollArea: HTMLElement | null
  tocScrollArea: HTMLElement | null
}

const LONG_PILL_THRESHOLD = 24

const state: TocSidebarState = {
  links: [],
  activeIds: [],
  headings: [],
  regions: [],
  scrollArea: null,
  tocScrollArea: null,
}

let eventController: AbortController | null = null
let visibilityObserver: MutationObserver | null = null
let titleObserver: IntersectionObserver | null = null
let scrollMaskTimer: number | null = null
let lifecycleReady = false

function resetState(): void {
  const tocContainer = document.getElementById('toc-sidebar-container')

  state.links = Array.from(
    document.querySelectorAll<HTMLElement>('#toc-sidebar-container [data-heading-link]'),
  )
  state.activeIds = []
  state.headings = []
  state.regions = []
  state.tocScrollArea = tocContainer?.querySelector<HTMLElement>('[data-toc-scroll-area]') ?? null
  state.scrollArea = findScrollAreaViewport(state.tocScrollArea ?? tocContainer)
}

function buildRegions(): void {
  state.headings = getContentHeadings()
  state.regions = buildHeadingRegions(state.headings)
}

function visibleHeadingIds(): string[] {
  return getVisibleHeadingIds(state.headings, state.regions)
}

function updateScrollMask(): void {
  if (!state.scrollArea || !state.tocScrollArea) return

  setVerticalScrollMask(state.scrollArea, state.tocScrollArea, {
    top: 'mask-t-from-90%',
    bottom: 'mask-b-from-90%',
  })
}

function linkForHeading(headingId: string): HTMLElement | null {
  return state.links.find((link) => link.dataset.headingLink === headingId) ?? null
}

function scrollToActiveHeading(headingIds: string[]): void {
  if (!state.scrollArea || headingIds.length === 0) return

  const activeLink = linkForHeading(headingIds[0])
  if (activeLink) centerElementInScrollContainer(state.scrollArea, activeLink)
}

function updateActiveLinks(headingIds: string[]): void {
  state.links.forEach((link) => link.classList.remove('text-foreground'))

  headingIds.forEach((id) => {
    linkForHeading(id)?.classList.add('text-foreground')
  })

  scrollToActiveHeading(headingIds)
}

function syncItemVisibility(visibleHeadings: HTMLElement[]): void {
  const visibleIds = new Set(visibleHeadings.map((heading) => heading.id))

  document
    .querySelectorAll<HTMLElement>('#toc-sidebar-container [data-toc-item]')
    .forEach((item) => {
      const slug = item.dataset.tocItem
      item.hidden = !slug || !visibleIds.has(slug)
    })
}

function syncHeadingLabels(): void {
  state.links.forEach((link) => {
    const slug = link.dataset.headingLink
    if (!slug) return

    const heading = document.getElementById(slug)
    const target = link.querySelector<HTMLElement>('[data-toc-text]')
    if (!heading || !target) return

    target.innerHTML = headingHtml(heading, {
      longPillThreshold: LONG_PILL_THRESHOLD,
    })
  })
}

function cleanupTitleVisibility(): void {
  titleObserver?.disconnect()
  titleObserver = null
}

function setupTitleVisibility(): void {
  const wrapper = document.getElementById('toc-sidebar-title-wrapper')
  const heading = document.getElementById('post-title')
  if (!wrapper || !heading) return

  cleanupTitleVisibility()

  titleObserver = new IntersectionObserver(
    (entries) => {
      const entry = entries[0]
      if (!entry) return

      wrapper.dataset.open =
        !entry.isIntersecting && entry.boundingClientRect.bottom <= 0 ? 'true' : 'false'
    },
    { rootMargin: '0px', threshold: 0 },
  )

  titleObserver.observe(heading)
}

function handleContentScroll(): void {
  const newActiveIds = visibleHeadingIds()

  if (!sameStringArray(newActiveIds, state.activeIds)) {
    state.activeIds = newActiveIds
    updateActiveLinks(state.activeIds)
  }
}

function handleResize(): void {
  buildRegions()
  const newActiveIds = visibleHeadingIds()

  if (!sameStringArray(newActiveIds, state.activeIds)) {
    state.activeIds = newActiveIds
    updateActiveLinks(state.activeIds)
  }

  syncItemVisibility(state.headings)
  updateScrollMask()
}

function observeVisibilityChanges(): void {
  const root = document.querySelector('.prose')
  if (!root) return

  visibilityObserver = new MutationObserver((mutations) => {
    const relevant = mutations.some(
      (mutation) =>
        mutation.type === 'attributes' &&
        (mutation.attributeName === 'hidden' || mutation.attributeName === 'open'),
    )

    if (relevant) handleResize()
  })

  visibilityObserver.observe(root, {
    subtree: true,
    attributes: true,
    attributeFilter: ['hidden', 'open'],
  })
}

function cleanupTocSidebar(): void {
  eventController?.abort()
  eventController = null

  visibilityObserver?.disconnect()
  visibilityObserver = null
  cleanupTitleVisibility()

  if (scrollMaskTimer !== null) {
    window.clearTimeout(scrollMaskTimer)
    scrollMaskTimer = null
  }

  Object.assign(state, {
    links: [],
    activeIds: [],
    headings: [],
    regions: [],
    scrollArea: null,
    tocScrollArea: null,
  })
}

function initTocSidebar(): void {
  cleanupTocSidebar()
  resetState()

  let hasHeadings = false

  withDatasetFlag('[data-toc-sidebar-root]', 'tocSettling', () => {
    syncHeadingLabels()
    buildRegions()
    setupTitleVisibility()
    hasHeadings = state.headings.length > 0

    if (!hasHeadings) {
      updateActiveLinks([])
      syncItemVisibility([])
      return
    }

    handleContentScroll()
    syncItemVisibility(state.headings)
  })

  if (!hasHeadings) return

  eventController = new AbortController()
  const { signal } = eventController

  scrollMaskTimer = window.setTimeout(updateScrollMask, 100)

  window.addEventListener('scroll', handleContentScroll, { passive: true, signal })
  window.addEventListener('resize', handleResize, { passive: true, signal })
  state.scrollArea?.addEventListener('scroll', updateScrollMask, { passive: true, signal })
  observeVisibilityChanges()
}

export function mountTocSidebar(): void {
  initTocSidebar()

  if (lifecycleReady) return
  lifecycleReady = true

  document.addEventListener('astro:before-swap', cleanupTocSidebar)
  document.addEventListener('astro:after-swap', initTocSidebar)
}
