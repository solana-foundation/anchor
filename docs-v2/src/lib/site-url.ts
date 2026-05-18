export const DEFAULT_SITE_URL = 'https://www.anchor-lang.com'

type Environment = Record<string, string | undefined>

export function resolveSiteUrl(env: Environment = process.env): string {
  return (
    normalizeSiteUrl(env.SITE_URL) ??
    normalizeSiteUrl(env.PUBLIC_SITE_URL) ??
    normalizeSiteUrl(env.ASTRO_SITE) ??
    resolveVercelSiteUrl(env) ??
    DEFAULT_SITE_URL
  )
}

function resolveVercelSiteUrl(env: Environment): string | null {
  if (env.VERCEL !== '1') return null

  if (env.VERCEL_ENV === 'production') {
    return normalizeSiteUrl(env.VERCEL_PROJECT_PRODUCTION_URL)
  }

  return normalizeSiteUrl(env.VERCEL_URL)
}

function normalizeSiteUrl(value: string | undefined): string | null {
  const trimmed = value?.trim()
  if (!trimmed) return null

  const withProtocol = /^https?:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`

  try {
    const url = new URL(withProtocol)
    return url.origin
  } catch {
    return null
  }
}
