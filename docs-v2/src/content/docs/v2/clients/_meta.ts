import type { MetaFile } from '@/types'

export default {
  label: 'Clients and IDL',
  items: {
    index: { label: 'Overview', order: 0 },
    rust: { order: 1 },
    typescript: { label: 'TypeScript', order: 2 },
  },
} satisfies MetaFile
