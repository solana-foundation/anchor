import type { DocsConfig, IconMap, Site, SocialLink } from '@/types'

export const SITE: Site = {
  title: 'Anchor Docs',
  description: 'Anchor is the leading development framework for building Solana programs.',
  href: 'https://www.anchor-lang.com',
  author: 'solana-foundation',
  locale: 'en-US',
}

export const SOCIAL_LINKS: SocialLink[] = [
  { href: 'https://github.com/solana-foundation/anchor', label: 'GitHub' },
  { href: 'https://discord.com/invite/NHHGSXAnXk', label: 'Discord' },
]

export const ICON_MAP: IconMap = {
  Website: 'lucide:globe',
  GitHub: 'lucide:github',
  LinkedIn: 'lucide:linkedin',
  Twitter: 'lucide:twitter',
  Email: 'lucide:mail',
  RSS: 'lucide:rss',
  Discord: 'lucide:message-circle',
}

export const DOCS: DocsConfig = {
  repoUrl: 'https://github.com/solana-foundation/anchor',
  editUrlBase:
    'https://github.com/solana-foundation/anchor/edit/anchor-next/docs-v2/src/content/docs/',
  defaultEditUrl: true,
  defaultLastUpdated: true,
  defaultTableOfContents: { minDepth: 2, maxDepth: 4 },
  search: {
    enabled: true,
    hotkey: { mac: '⌘ K', windows: 'Ctrl K' },
  },
  announcement: null,
}
