import { glob } from 'astro/loaders'
import { defineCollection } from 'astro:content'
import { z } from 'astro/zod'

const badgeShorthand = z.enum(['new', 'beta', 'deprecated', 'soon'])
const badgeFull = z.object({
  text: z.string(),
  variant: z.enum(['default', 'note', 'tip', 'caution', 'danger']).optional(),
})
const sidebarBadge = z.union([badgeShorthand, badgeFull])

const docs = defineCollection({
  loader: glob({ pattern: '**/*.{md,mdx}', base: './src/content/docs' }),
  schema: () =>
    z.object({
      title: z.string().max(100),
      description: z.string().max(200).optional(),
      sidebar: z
        .object({
          order: z.number().optional(),
          label: z.string().optional(),
          badge: sidebarBadge.optional(),
          hidden: z.boolean().default(false),
        })
        .optional(),
      editUrl: z.union([z.url(), z.literal(false)]).optional(),
      lastUpdated: z.union([z.boolean(), z.coerce.date()]).optional(),
      prev: z
        .union([z.object({ label: z.string(), link: z.string() }), z.literal(false)])
        .optional(),
      next: z
        .union([z.object({ label: z.string(), link: z.string() }), z.literal(false)])
        .optional(),
      tableOfContents: z
        .union([
          z.boolean(),
          z.object({
            minDepth: z.number().int().min(1).max(6).default(2),
            maxDepth: z.number().int().min(1).max(6).default(4),
          }),
        ])
        .optional(),
      banner: z.string().optional(),
      wide: z.boolean().default(false),
      pageHeader: z.boolean().default(true),
      autoCards: z.boolean().default(true),
      draft: z.boolean().default(false),
    }),
})

export const collections = { docs }
