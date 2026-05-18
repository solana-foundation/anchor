const TAB_ROOT_SELECTOR = '[data-tabs]'
const TAB_SELECTOR = ':scope > [role="tablist"] > [data-tab-index]'
const PANEL_WRAPPER_SELECTOR = ':scope > [data-tabs-panels]'

let lifecycleReady = false

function clampIndex(index: number, length: number): number {
  if (!Number.isFinite(index)) return 0
  return Math.max(0, Math.min(index, Math.max(0, length - 1)))
}

function setupTabRoot(root: HTMLElement): void {
  if (root.dataset.tabsInit === '1') return
  root.dataset.tabsInit = '1'

  // `:scope >` keeps nested <Tabs> descendants from being matched by the
  // outer tabs controller.
  const tabs = root.querySelectorAll<HTMLButtonElement>(TAB_SELECTOR)
  const panelsWrapper = root.querySelector<HTMLElement>(PANEL_WRAPPER_SELECTOR)
  if (!panelsWrapper) return

  const panels = Array.from(panelsWrapper.children).filter(
    (panel): panel is HTMLElement => panel instanceof HTMLElement,
  )
  const defaultIndex = clampIndex(Number(panelsWrapper.dataset.defaultIndex ?? 0), panels.length)

  const select = (index: number) => {
    const selectedIndex = clampIndex(index, panels.length)

    tabs.forEach((tab, i) => {
      const active = i === selectedIndex
      tab.setAttribute('aria-selected', String(active))
      tab.dataset.selected = String(active)
      tab.tabIndex = active ? 0 : -1
    })

    panels.forEach((panel, i) => {
      panel.hidden = i !== selectedIndex
    })
  }

  select(defaultIndex)

  tabs.forEach((tab, index) => {
    tab.addEventListener('click', () => select(index))
    tab.addEventListener('keydown', (event) => {
      const isRight = event.key === 'ArrowRight'
      const isLeft = event.key === 'ArrowLeft'
      if (!isRight && !isLeft) return

      event.preventDefault()
      const tabId = (isRight ? index + 1 : index - 1 + tabs.length) % tabs.length
      tabs[tabId]?.focus()
      select(tabId)
    })
  })
}

function resetTabs(): void {
  document.querySelectorAll<HTMLElement>(TAB_ROOT_SELECTOR).forEach((root) => {
    delete root.dataset.tabsInit
  })
}

export function setupTabs(): void {
  document.querySelectorAll<HTMLElement>(TAB_ROOT_SELECTOR).forEach(setupTabRoot)
}

export function mountTabs(): void {
  setupTabs()

  if (lifecycleReady) return
  lifecycleReady = true

  document.addEventListener('astro:after-swap', () => {
    resetTabs()
    setupTabs()
  })
}
