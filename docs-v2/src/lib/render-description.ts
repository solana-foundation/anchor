import { codeToHtml } from 'shiki'

import { latte, mocha } from './shiki-themes'

const PATTERN = /`([^`]+?)(?:\{:([a-z0-9]+)\})?`/g

function escapeHtml(text: string): string {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}

export interface RenderedDescription {
  html: string
  plain: string
}

export async function renderDescription(text: string): Promise<RenderedDescription> {
  const plain = text.replace(PATTERN, (_, code) => code)

  const re = new RegExp(PATTERN.source, 'g')
  const parts: string[] = []
  let lastIndex = 0
  let match: RegExpExecArray | null

  while ((match = re.exec(text)) !== null) {
    const [full, code, lang] = match
    if (match.index > lastIndex) {
      parts.push(escapeHtml(text.slice(lastIndex, match.index)))
    }
    if (lang) {
      const rendered = await codeToHtml(code, {
        lang,
        themes: { light: latte, dark: mocha },
        structure: 'inline',
      })
      parts.push(`<span class="shiki">${rendered}</span>`)
    } else {
      parts.push(`<code>${escapeHtml(code)}</code>`)
    }
    lastIndex = match.index + full.length
  }

  if (lastIndex < text.length) {
    parts.push(escapeHtml(text.slice(lastIndex)))
  }

  return { html: parts.join(''), plain }
}
