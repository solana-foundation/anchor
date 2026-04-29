import type { MetaFile } from '@/types'

export default {
  label: 'Anchor v2 (alpha)',
  items: {
    index: { label: 'Overview', order: 0 },
    installation: { order: 1 },
    quickstart: { order: 2 },
    migration: { label: 'Migrating from v1', order: 3 },
    'account-types': { order: 4 },
    'pod-types': { order: 5 },
    macros: { label: 'Macros and derives', order: 6 },
    cpi: { label: 'CPI mechanics', order: 7 },
    optimizations: { order: 8 },
    extensibility: { order: 9 },
    'testing-and-debugging': { order: 10 },
    'feature-flags': { order: 11 },
  },
} satisfies MetaFile
