import { glob } from 'astro/loaders'
import { defineCollection } from 'astro:content'
import { z } from 'astro/zod'

const badgeShorthand = z.enum(['new', 'beta', 'deprecated', 'soon'])
const badgeFull = z.object({
  text: z.string(),
  variant: z.enum(['default', 'note', 'tip', 'caution', 'danger']).optional(),
})
const sidebarBadge = z.union([badgeShorthand, badgeFull])

const heroAction = z.object({
  label: z.string(),
  link: z.string(),
  variant: z.enum(['primary', 'secondary', 'ghost']).default('primary'),
  icon: z.string().optional(),
  external: z.boolean().optional(),
})

const docs = defineCollection({
  loader: glob({ pattern: '**/*.{md,mdx}', base: './src/content/docs' }),
  schema: ({ image }) =>
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
      autoCards: z.boolean().default(true),
      template: z.enum(['doc', 'splash']).default('doc'),
      hero: z
        .object({
          title: z.string().optional(),
          tagline: z.string().optional(),
          image: image().optional(),
          actions: z.array(heroAction).default([]),
        })
        .optional(),
      draft: z.boolean().default(false),
    }),
})

export const collections = { docs }
