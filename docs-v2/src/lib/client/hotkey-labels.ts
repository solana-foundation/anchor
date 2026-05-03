type NavigatorWithUserAgentData = Navigator & {
  userAgentData?: {
    platform?: string
  }
}

let lifecycleReady = false

function isMacPlatform(): boolean {
  const nav = navigator as NavigatorWithUserAgentData
  const platform = nav.userAgentData?.platform ?? ''

  return /Mac|iPhone|iPad|iPod/i.test(`${navigator.userAgent} ${platform}`)
}

function applyOsHotkey(): void {
  const mac = isMacPlatform()

  document.querySelectorAll<HTMLElement>('[data-hotkey]').forEach((el) => {
    const label = mac ? (el.dataset.hotkeyMac ?? '') : (el.dataset.hotkeyOther ?? '')
    if (label) el.textContent = label
  })
}

export function mountHotkeyLabels(): void {
  applyOsHotkey()

  if (lifecycleReady) return
  lifecycleReady = true

  document.addEventListener('astro:after-swap', applyOsHotkey)
}
