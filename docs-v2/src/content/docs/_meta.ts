import type { MetaFile } from '@/types'

export default {
  items: {
    index: { label: 'Docs home', order: 0 },
    v1: { label: 'Anchor v1', order: 1 },
    v2: { label: 'Anchor v2 (alpha)', order: 2 },
    updates: { order: 3 },
  },
} satisfies MetaFile
