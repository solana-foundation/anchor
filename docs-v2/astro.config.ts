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
import { ecOptions } from './src/lib/ec-config'
import { latte, mocha } from './src/lib/shiki-themes'

import tailwindcss from '@tailwindcss/vite'
import { extname, resolve } from 'node:path'
import { readFile } from 'node:fs/promises'

/**
 * Dev-only: serve `/pagefind/*` from `./dist/pagefind/*`.
 *
 * Pagefind writes its index into the built `dist/` folder, but Astro's
 * dev server only serves `public/` and source. Without this plugin the
 * search dialog 404s on pagefind.js during `bun run dev`, even after a
 * successful `bun run build`.
 */
function rehypeWrapTables() {
  return (tree: any) => {
    const visit = (node: any, parent: any, index: number | null) => {
      if (node.type === 'element' && node.tagName === 'table' && parent && index !== null) {
        const wrapper = {
          type: 'element',
          tagName: 'div',
          properties: { className: ['table-wrapper'] },
          children: [node],
        }
        parent.children[index] = wrapper
        return
      }
      if (node.children) {
        for (let i = 0; i < node.children.length; i++) {
          visit(node.children[i], node, i)
        }
      }
    }
    visit(tree, null, null)
  }
}

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
    configureServer(server: any) {
      server.middlewares.use('/docs/pagefind', async (req: any, res: any, next: any) => {
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
  base: '/docs',
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
