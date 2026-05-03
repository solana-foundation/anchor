import { defineConfig } from 'astro/config'

import mdx from '@astrojs/mdx'
import react from '@astrojs/react'
import sitemap from '@astrojs/sitemap'

import { rehypeHeadingIds } from '@astrojs/markdown-remark'
import rehypeAutolinkHeadings from 'rehype-autolink-headings'
import rehypeExpressiveCode from 'rehype-expressive-code'
import rehypeExternalLinks from 'rehype-external-links'
import rehypeKatex from 'rehype-katex'
import rehypeShiki from '@shikijs/rehype'
import remarkEmoji from 'remark-emoji'
import remarkMath from 'remark-math'

import { rehypeInlineShellCmd } from './src/lib/rehype-inline-shell-cmd'
import { rehypeInlinePathIcon } from './src/lib/rehype-inline-path-icon'
import { rehypeWrapTables } from './src/lib/rehype-wrap-tables'
import { ecOptions } from './src/lib/ec-config'
import { latte, mocha } from './src/lib/shiki-themes'

import tailwindcss from '@tailwindcss/vite'
import { extname, resolve } from 'node:path'
import { readFile } from 'node:fs/promises'

type DevMiddleware = (
  req: { url?: string },
  res: { setHeader(name: string, value: string): void; end(data: Uint8Array): void },
  next: () => void,
) => void | Promise<void>

type DevServer = {
  middlewares: {
    use(path: string, handler: DevMiddleware): void
  }
}

const DOCS_BASE = '/docs'
const PAGEFIND_DEV_PATH = `${DOCS_BASE}/pagefind`

/**
 * Dev-only: serve `/docs/pagefind/*` from `./dist/docs/pagefind/*`.
 *
 * Pagefind writes its index into the built `dist/` folder, but Astro's
 * dev server only serves `public/` and source. Without this plugin the
 * search dialog 404s on pagefind.js during `bun run dev`, even after a
 * successful `bun run build`.
 */
function pagefindDevServer() {
  const mime: Record<string, string> = {
    js: 'application/javascript',
    mjs: 'application/javascript',
    css: 'text/css',
    json: 'application/json',
    wasm: 'application/wasm',
  }
  return {
    name: 'pagefind-dev-server',
    apply: 'serve' as const,
    configureServer(server: DevServer) {
      server.middlewares.use(PAGEFIND_DEV_PATH, async (req, res, next) => {
        const url = (req.url ?? '/').split('?')[0]
        if (url === '' || url === '/') return next()
        try {
          const filePath = resolve(process.cwd(), 'dist', 'docs', 'pagefind' + url)
          const data = await readFile(filePath)
          const ext = extname(url).slice(1)
          if (mime[ext]) res.setHeader('Content-Type', mime[ext])
          res.end(data)
        } catch {
          next()
        }
      })
    },
  }
}

export default defineConfig({
  site: 'https://www.anchor-lang.com',
  base: DOCS_BASE,
  trailingSlash: 'always',
  outDir: './dist/docs',
  integrations: [mdx(), react(), sitemap()],
  vite: {
    plugins: [tailwindcss(), pagefindDevServer()],
  },
  server: {
    port: 4321,
    host: true,
  },
  devToolbar: {
    enabled: false,
  },
  markdown: {
    syntaxHighlight: false,
    rehypePlugins: [
      [
        rehypeExternalLinks,
        {
          target: '_blank',
          rel: ['nofollow', 'noreferrer', 'noopener'],
        },
      ],
      rehypeWrapTables,
      rehypeKatex,
      [rehypeExpressiveCode, { themes: [latte, mocha], ...ecOptions }],
      [
        rehypeShiki,
        {
          themes: { light: latte, dark: mocha },
          inline: 'tailing-curly-colon',
        },
      ],
      rehypeInlineShellCmd,
      rehypeInlinePathIcon,
      rehypeHeadingIds,
      [
        rehypeAutolinkHeadings,
        {
          behavior: 'append',
          properties: {
            className: ['heading-anchor'],
            'aria-label': 'Link to section',
            tabindex: -1,
            'data-pagefind-ignore': '',
          },
          content: {
            type: 'text',
            value: '#',
          },
          test: (node: { tagName: string }) =>
            ['h2', 'h3', 'h4', 'h5', 'h6'].includes(node.tagName),
        },
      ],
    ],
    remarkPlugins: [remarkMath, remarkEmoji],
  },
})
