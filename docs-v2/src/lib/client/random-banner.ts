type BannerOption = {
  id: string
  src: string
  title: string
  artist: string
  description: string
  objectPosition: string
  weight?: number
}

type AstroBeforeSwapEvent = Event & {
  from: string
  to: string
  newDocument: Document
}

const BASE_PATH = import.meta.env.BASE_URL.replace(/\/$/, '')
const BANNER_SELECTOR = '[data-doc-banner][data-random-banner="true"]'

let listenersReady = false

function isBannerOption(value: unknown): value is BannerOption {
  if (!value || typeof value !== 'object') return false

  const option = value as Partial<Record<keyof BannerOption, unknown>>
  return (
    typeof option.id === 'string' &&
    typeof option.src === 'string' &&
    typeof option.title === 'string' &&
    typeof option.artist === 'string' &&
    typeof option.description === 'string' &&
    typeof option.objectPosition === 'string'
  )
}

function parseBannerOptions(value: string | null): BannerOption[] {
  if (!value) return []

  try {
    const parsed = JSON.parse(value)
    return Array.isArray(parsed) ? parsed.filter(isBannerOption) : []
  } catch {
    return []
  }
}

function optionWeight(option: BannerOption): number {
  const weight = Number(option.weight)
  return Number.isFinite(weight) && weight > 0 ? weight : 0
}

function pickWeightedBanner(options: BannerOption[]): BannerOption {
  const totalWeight = options.reduce((sum, option) => sum + optionWeight(option), 0)
  if (totalWeight <= 0) return options[0]

  const random = Math.random() * totalWeight
  let cursor = 0

  return (
    options.find((option) => {
      cursor += optionWeight(option)
      return random <= cursor
    }) ?? options[0]
  )
}

function applyBanner(banner: Element, selected: BannerOption): void {
  const image = banner.querySelector('[data-banner-image]')
  const title = banner.querySelector('[data-banner-title]')
  const artist = banner.querySelector('[data-banner-artist]')

  if (banner instanceof HTMLElement) {
    banner.dataset.bannerGraphicId = selected.id
  }

  if (image instanceof HTMLImageElement) {
    image.src = selected.src
    image.alt = selected.description
    image.title = `${selected.title} by ${selected.artist}`
    image.style.objectPosition = selected.objectPosition
  }

  if (title) title.textContent = selected.title
  if (artist) artist.textContent = selected.artist
}

function isDocsHomeUrl(url: string): boolean {
  const pathname = new URL(url, window.location.href).pathname.replace(/\/+$/, '')
  return pathname === BASE_PATH
}

function isHomeDocsTransition(fromUrl: string, toUrl: string): boolean {
  return isDocsHomeUrl(fromUrl) !== isDocsHomeUrl(toUrl)
}

export function setupRandomDocBanners(root: ParentNode = document, preferredGraphicId = ''): void {
  const banners = root.querySelectorAll(BANNER_SELECTOR)

  banners.forEach((banner) => {
    const options = parseBannerOptions(banner.getAttribute('data-banner-options'))
    if (options.length === 0) return

    const currentId = banner instanceof HTMLElement ? banner.dataset.bannerGraphicId : ''
    const selected =
      options.find((option) => option.id === preferredGraphicId) ??
      options.find((option) => option.id === currentId) ??
      pickWeightedBanner(options)

    applyBanner(banner, selected)
  })
}

function beforeSwap(event: Event): void {
  const { from, to, newDocument } = event as AstroBeforeSwapEvent
  const currentBanner = document.querySelector<HTMLElement>('[data-doc-banner]')
  const preferredGraphicId = isHomeDocsTransition(from, to)
    ? (currentBanner?.dataset.bannerGraphicId ?? '')
    : ''

  setupRandomDocBanners(newDocument, preferredGraphicId)
}

export function mountRandomDocBanners(): void {
  setupRandomDocBanners()

  if (listenersReady) return
  listenersReady = true

  document.addEventListener('astro:before-swap', beforeSwap)
  document.addEventListener('astro:after-swap', () => setupRandomDocBanners())
}
