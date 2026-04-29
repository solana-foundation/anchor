import type { MetaFile } from '@/types'

export default {
  label: 'Program development',
  items: {
    'account-types': { order: 0 },
    'account-constraints': { order: 1 },
    'anchor-toml': { order: 2 },
    cli: { order: 3 },
    avm: { order: 4 },
    space: { order: 5 },
    'type-conversion': { order: 6 },
    'verifiable-builds': { order: 7 },
    'security-exploits': { order: 8 },
    footguns: { order: 9 },
    examples: { order: 10 },
  },
} satisfies MetaFile
