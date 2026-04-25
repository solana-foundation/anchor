import type { MetaFile } from '@/types'

export default {
  label: 'Additional features',
  items: {
    'declare-program': { order: 0 },
    errors: { order: 1 },
    events: { order: 2 },
    'zero-copy': { order: 3 },
  },
} satisfies MetaFile
