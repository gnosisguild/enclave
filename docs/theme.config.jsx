// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { Link, useConfig } from 'nextra-theme-docs'
import { useRouter } from 'next/router'
import Footer from './components/Footer'

export default {
  logo: (
    <Link href='https://enclave.gg' target='_self'>
      <img src='/enclave-logo-solid.svg' style={{ maxWidth: '150px' }} />
    </Link>
  ),
  logoLink: false,
  project: {
    link: 'https://github.com/gnosisguild/enclave',
  },
  docsRepositoryBase: 'https://github.com/gnosisguild/enclave-docs',
  darkMode: false,
  nextThemes: {
    defaultTheme: 'light',
  },
  primaryHue: 203,
  primarySaturation: 100,

  sidebar: {
    defaultMenuCollapseLevel: 1,
  },
  useNextSeoProps() {
    const { asPath } = useRouter()
    if (asPath !== '/') {
      return {
        titleTemplate: '%s - Enclave',
      }
    }
  },
  head: function useHead() {
    const {
      frontMatter: { title, description },
    } = useConfig()
    return (
      <>
        <title>{title ? title : 'Enclave'}</title>
        <meta name='title' content={title ? title : 'Enclave'} />
        <meta
          name='description'
          content={
            description
              ? `${description}`
              : 'An open-source protocol for Encrypted Execution Environments (E3) enabling a new class of secure applications.'
          }
        />

        <meta property='og:type' content='website' />
        <meta property='og:url' content='https://docs.enclave.gg' />
        <meta property='og:title' content={title ? title : 'Enclave'} />
        <meta
          property='og:description'
          content={
            description
              ? `${description}`
              : 'An open-source protocol for Encrypted Execution Environments (E3) enabling a new class of secure applications.'
          }
        />
        <meta property='og:image' content='https://docs.enclave.gg/enclave-meta.jpg' />

        <meta property='twitter:card' content='summary_large_image' />
        <meta property='twitter:url' content='https://docs.enclave.gg' />
        <meta property='twitter:title' content={title ? title : 'Enclave'} />
        <meta
          property='twitter:description'
          content={
            description
              ? `${description}`
              : 'An open-source protocol for Encrypted Execution Environments (E3) enabling a new class of secure applications.'
          }
        />
        <meta property='twitter:image' content='/enclave-meta.jpg' />

        <link rel='apple-touch-icon' sizes='180x180' href='/apple-touch-icon.png' />
        <link rel='icon' type='image/png' sizes='32x32' href='/favicon-32x32.png' />
        <link rel='icon' type='image/png' sizes='16x16' href='/favicon-16x16.png' />
        <link rel='manifest' href='/site.webmanifest' />
        <link rel='mask-icon' href='/safari-pinned-tab.svg' color='#5bbad5' />
        <meta name='msapplication-TileColor' content='#da532c' />
        <meta name='theme-color' content='#ffffff' />
      </>
    )
  },
  footer: {
    component: <Footer />,
  },
  // ... other theme options
}
