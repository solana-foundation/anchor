import { definePlugin, type ExpressiveCodePlugin } from '@expressive-code/core'
import { select, selectAll, type Element, type ElementContent } from '@expressive-code/core/hast'

const TERMINAL_LANGS = new Set([
  'ansi',
  'bash',
  'sh',
  'shell',
  'shellscript',
  'shellsession',
  'zsh',
  'console',
])

function hasClass(node: Element, className: string): boolean {
  const classes = node.properties?.className
  return Array.isArray(classes) && classes.includes(className)
}

function addClass(node: Element, className: string): void {
  const classes = node.properties?.className
  const arr = Array.isArray(classes) ? [...classes] : []
  if (!arr.includes(className)) arr.push(className)
  node.properties = { ...node.properties, className: arr }
}

function extractText(node: ElementContent): string {
  if (node.type === 'text') return node.value
  if (node.type !== 'element' || !node.children) return ''
  return node.children.map(extractText).join('')
}

function extractCommandText(codeNode: Element): string {
  const parts: string[] = []
  for (const child of codeNode.children) {
    if (child.type === 'element' && hasClass(child, 'shell-prompt')) continue
    parts.push(extractText(child))
  }
  return parts.join('')
}

export function pluginOutputSeparator(): ExpressiveCodePlugin {
  return definePlugin({
    name: 'Output Separator',
    baseStyles: `
      .ec-line.ec-cmd + .ec-line.ec-out,
      .ec-line.ec-out + .ec-line.ec-cmd {
        border-top: 2px solid var(--border);
        margin-top: 0.75rem;
        padding-top: 0.75rem;
      }
    `,
    hooks: {
      postprocessRenderedBlock: ({ codeBlock, renderData }) => {
        if (!TERMINAL_LANGS.has(codeBlock.language)) return

        const lines = selectAll('div.ec-line', renderData.blockAst)
        if (lines.length === 0) return

        const commands: string[] = []
        let hasCmd = false
        let hasOut = false

        for (const line of lines) {
          const codeNode = select('div.code', line)
          const isCmd = !!select('span.shell-prompt', line)
          if (isCmd) {
            addClass(line, 'ec-cmd')
            hasCmd = true
            if (codeNode) commands.push(extractCommandText(codeNode))
          } else {
            addClass(line, 'ec-out')
            hasOut = true
          }
        }

        if (!hasCmd || !hasOut) return

        const copyButton = select('.copy button', renderData.blockAst)
        if (copyButton) {
          copyButton.properties = {
            ...copyButton.properties,
            'data-code': commands.join('\u007f'),
          }
        }
      },
    },
  })
}
