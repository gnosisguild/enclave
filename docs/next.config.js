// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// eslint-disable-next-line @typescript-eslint/no-require-imports
const nextra = require('nextra')

const withNextra = nextra({
  theme: 'nextra-theme-docs',
  themeConfig: './theme.config.jsx',
})

module.exports = withNextra({
  async redirects() {
    return [
      {
        source: '/',
        destination: '/introduction',
        permanent: false,
      },
    ]
  },
})
