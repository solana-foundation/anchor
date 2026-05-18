const KATEX_STYLESHEET_URL = 'https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.css'

let lifecycleReady = false

function hasKatexMarkup(): boolean {
  return Boolean(document.querySelector('.katex'))
}

function hasKatexStylesheet(): boolean {
  return Boolean(document.querySelector('link[data-katex-stylesheet]'))
}

function ensureKatexStyles(): void {
  if (!hasKatexMarkup() || hasKatexStylesheet()) return

  const link = document.createElement('link')
  link.rel = 'stylesheet'
  link.href = KATEX_STYLESHEET_URL
  link.dataset.katexStylesheet = 'true'
  document.head.appendChild(link)
}

export function mountKatexStyles(): void {
  ensureKatexStyles()

  if (lifecycleReady) return
  lifecycleReady = true

  document.addEventListener('astro:page-load', ensureKatexStyles)
  document.addEventListener('astro:after-swap', ensureKatexStyles)
}
