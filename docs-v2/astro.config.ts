import { defineConfig } from 'astro/config'

import mdx from '@astrojs/mdx'
import react from '@astrojs/react'
import sitemap from '@astrojs/sitemap'
import icon from 'astro-icon'

import { rehypeHeadingIds } from '@astrojs/markdown-remark'
import rehypeAutolinkHeadings from 'rehype-autolink-headings'
import rehypeExpressiveCode from 'rehype-expressive-code'
import rehypeExternalLinks from 'rehype-external-links'
import rehypeKatex from 'rehype-katex'
import rehypeShiki from '@shikijs/rehype'
import remarkEmoji from 'remark-emoji'
import remarkMath from 'remark-math'

import { pluginCollapsibleSections } from '@expressive-code/plugin-collapsible-sections'
import { pluginLineNumbers } from '@expressive-code/plugin-line-numbers'
import type { ExpressiveCodeTheme } from 'rehype-expressive-code'

import { pluginShellPrompt } from './src/lib/ec-shell-prompt'
import { pluginOutputSeparator } from './src/lib/ec-output-separator'

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
      server.middlewares.use('/pagefind', async (req: any, res: any, next: any) => {
        const url = (req.url ?? '/').split('?')[0]
        if (url === '' || url === '/') return next()
        try {
          const filePath = resolve(process.cwd(), 'dist', 'pagefind' + url)
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
  integrations: [mdx(), react(), sitemap(), icon()],
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
      rehypeHeadingIds,
      rehypeWrapTables,
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
      rehypeKatex,
      [
        rehypeExpressiveCode,
        {
          themes: ['catppuccin-latte', 'catppuccin-mocha'],
          plugins: [
            pluginCollapsibleSections(),
            pluginLineNumbers(),
            pluginShellPrompt(),
            pluginOutputSeparator(),
          ],
          useDarkModeMediaQuery: false,
          themeCssSelector: (theme: ExpressiveCodeTheme) =>
            `[data-theme="${theme.name === 'catppuccin-latte' ? 'light' : 'dark'}"]`,
          defaultProps: {
            wrap: true,
            showLineNumbers: true,
            collapseStyle: 'collapsible-auto',
            overridesByLang: {
              'ansi,bat,bash,batch,cmd,console,powershell,ps,ps1,psd1,psm1,sh,shell,shellscript,shellsession,text,zsh':
                { showLineNumbers: false },
              'yaml,yml,toml,json,json5,jsonc,sql,graphql,markdown,mdx': { showLineNumbers: false },
            },
          },
          styleOverrides: {
            codeFontSize: '0.75rem',
            borderColor: 'var(--border)',
            borderWidth: '2px',
            codeFontFamily: 'var(--font-mono)',
            codeBackground: ({ theme }: { theme: ExpressiveCodeTheme }) =>
              theme.name === 'catppuccin-latte' ? 'oklch(96% 0.008 286)' : 'oklch(24% 0.03 284)',
            frames: {
              editorActiveTabForeground: 'var(--muted-foreground)',
              editorActiveTabBackground: ({ theme }: { theme: ExpressiveCodeTheme }) =>
                theme.name === 'catppuccin-latte' ? 'oklch(96% 0.008 286)' : 'oklch(24% 0.03 284)',
              editorActiveTabIndicatorBottomColor: 'transparent',
              editorActiveTabIndicatorTopColor: 'transparent',
              editorTabBarBackground: 'transparent',
              editorTabBarBorderBottomColor: 'transparent',
              frameBoxShadowCssValue: 'none',
              terminalBackground: ({ theme }: { theme: ExpressiveCodeTheme }) =>
                theme.name === 'catppuccin-latte' ? 'oklch(96% 0.008 286)' : 'oklch(24% 0.03 284)',
              terminalTitlebarBackground: 'transparent',
              terminalTitlebarBorderBottomColor: 'transparent',
              terminalTitlebarForeground: 'var(--muted-foreground)',
            },
            lineNumbers: {
              foreground: 'var(--muted-foreground)',
            },
            collapsibleSections: {
              closedBackgroundColor: 'color-mix(in oklab, var(--accent) 14%, transparent)',
              closedBorderColor: 'color-mix(in oklab, var(--accent) 45%, transparent)',
              closedTextColor: 'var(--muted-foreground)',
              openBackgroundColorCollapsible: 'color-mix(in oklab, var(--accent) 7%, transparent)',
              openBorderColor: 'transparent',
            },
            textMarkers: {
              delBackground: 'color-mix(in oklab, var(--ctp-red) 22%, transparent)',
              delBorderColor: 'color-mix(in oklab, var(--ctp-red) 65%, transparent)',
              delDiffIndicatorColor: 'var(--ctp-red)',
              insBackground: 'color-mix(in oklab, var(--ctp-green) 22%, transparent)',
              insBorderColor: 'color-mix(in oklab, var(--ctp-green) 65%, transparent)',
              insDiffIndicatorColor: 'var(--ctp-green)',
              markBackground: 'color-mix(in oklab, var(--accent) 28%, transparent)',
              markBorderColor: 'var(--accent)',
            },
            uiFontFamily: 'var(--font-sans)',
          },
        },
      ],
      [
        rehypeShiki,
        {
          themes: {
            light: 'catppuccin-latte',
            dark: 'catppuccin-mocha',
          },
          inline: 'tailing-curly-colon',
        },
      ],
    ],
    remarkPlugins: [remarkMath, remarkEmoji],
  },
})
