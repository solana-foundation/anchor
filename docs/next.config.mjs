import { createMDX } from 'fumadocs-mdx/next';
import redirectsJson from './redirects.json' with { type: 'json' };

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  async redirects() {
    return [
      {
        source: '/',
        destination: '/docs',
        permanent: false,
      },
      ...redirectsJson.redirects.map((redirect) => ({
        ...redirect,
        permanent: redirect.permanent ?? true,
      })),
    ];
  },
  async rewrites() {
    return [
      {
        source: '/docs/:path*.mdx',
        destination: '/llms.mdx/docs/:path*',
      },
    ];
  },
};

export default withMDX(config);
