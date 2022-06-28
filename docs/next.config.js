const withMarkdoc = require('@markdoc/next.js')
const { withPlausibleProxy } = require('next-plausible')

/** @type {import('next').NextConfig} */
const nextConfig = withMarkdoc()({
  swcMinify: true,
  reactStrictMode: true,
  pageExtensions: ['js', 'jsx', 'md'],
  experimental: {
    newNextLinkBehavior: true,
    scrollRestoration: true,
    legacyBrowsers: false,
    images: { allowFutureImage: true },
  },
})

module.exports = withPlausibleProxy()(nextConfig)
