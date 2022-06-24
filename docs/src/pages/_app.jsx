import Head from 'next/head'
import { slugifyWithCounter } from '@sindresorhus/slugify'
import PlausibleProvider from 'next-plausible'

import Prism from 'prism-react-renderer/prism'
;(typeof global !== 'undefined' ? global : window).Prism = Prism

require('prismjs/components/prism-rust')
require('prismjs/components/prism-toml')

import { Layout } from '@/components/Layout'

import 'focus-visible'
import '@/styles/tailwind.css'

const navigation = [
  {
    title: 'Introduction',
    links: [
      { title: 'Getting started', href: '/' },
      { title: 'Installation', href: '/docs/installation' },
      { title: 'Hello World', href: '/docs/hello-world' },
      { title: 'Intro to Solana', href: '/docs/intro-to-solana' },
    ],
  },
  {
    title: 'Core concepts',
    links: [
      { title: 'High-level Overview', href: '/docs/high-level-overview' },
      {
        title: 'The Accounts Struct',
        href: '/docs/the-accounts-struct',
      },
      { title: 'The Program Module', href: '/docs/the-program-module' },
      {
        title: 'Errors',
        href: '/docs/errors',
      },
      {
        title: 'Cross-Program Invocations',
        href: '/docs/cross-program-invocations',
      },
      {
        title: 'Program Derived Addresses',
        href: '/docs/pdas',
      },
    ],
  },
  {
    title: 'Guides',
    links: [
      { title: 'Publishing Source', href: '/docs/publishing-source' },
      {
        title: 'Verifiable Builds',
        href: '/docs/verifiable-builds',
      },
    ],
  },
  {
    title: 'References',
    links: [
      { title: 'Space', href: '/docs/space' },
      {
        title: 'JavaScript Anchor Types',
        href: '/docs/javascript-anchor-types',
      },
      { title: 'CLI', href: '/docs/cli' },
      { title: 'AVM', href: '/docs/avm' },
      { title: 'Anchor.toml', href: '/docs/manifest' },
      {
        title: 'Important Links',
        href: '/docs/important-links',
      },
    ],
  },
  {
    title: 'Projects',
    links: [{ title: 'Tic-Tac-Toe', href: '/docs/tic-tac-toe' }],
  },
]

function getNodeText(node) {
  let text = ''
  for (let child of node.children ?? []) {
    if (typeof child === 'string') {
      text += child
    }
    text += getNodeText(child)
  }
  return text
}

function collectHeadings(nodes, slugify = slugifyWithCounter()) {
  let sections = []

  for (let node of nodes) {
    if (/^h[23]$/.test(node.name)) {
      let title = getNodeText(node)
      if (title) {
        let id = slugify(title)
        node.attributes.id = id
        if (node.name === 'h3') {
          sections[sections.length - 1].children.push({
            ...node.attributes,
            title,
          })
        } else {
          sections.push({ ...node.attributes, title, children: [] })
        }
      }
    }

    sections.push(...collectHeadings(node.children ?? [], slugify))
  }

  return sections
}

export default function App({ Component, pageProps }) {
  let title = pageProps.markdoc?.frontmatter.title

  let pageTitle =
    pageProps.markdoc?.frontmatter.pageTitle ||
    `${pageProps.markdoc?.frontmatter.title} - Docs`

  let description = pageProps.markdoc?.frontmatter.description

  let tableOfContents = pageProps.markdoc?.content
    ? collectHeadings(pageProps.markdoc.content)
    : []

  return (
    <>
      <PlausibleProvider domain="anchor-lang.com" trackOutboundLinks={true}>
        <Head>
          <title>{pageTitle}</title>
          {description && <meta name="description" content={description} />}
        </Head>
        <Layout
          navigation={navigation}
          title={title}
          tableOfContents={tableOfContents}
        >
          <Component {...pageProps} />
        </Layout>
      </PlausibleProvider>
    </>
  )
}
