export interface MenuItem {
  label: string
  href: string
  external?: boolean
  id?: string
}

export interface MenuConfig {
  main: MenuItem[]
  footer: MenuItem[]
  footerLinks: {
    project?: {
      name: string
      url: string
    }
    author?: {
      name: string
      url: string
    }
    website?: {
      name: string
      url: string
    }
  }
}

export const menuConfig: MenuConfig = {
  main: [
    {
      label: 'Home',
      href: '/',
      id: 'home-link'
    },
    {
      label: 'Tools',
      href: '/tools',
      id: 'tools-link'
    },
    {
      label: '/Vector Database',
      href: '/rag',
      id: 'rag-link'
    },
    {
      label: 'Agent Creator',
      href: '/agent-creator',
      id: 'footer-agent-link'
    }
  ],
  footer: [
    {
      label: 'Home',
      href: '/',
      id: 'footer-main-link'
    },
    {
      label: 'Tools',
      href: '/tools',
      id: 'footer-tools-link'
    },
    {
      label: 'Vector Database',
      href: '/rag',
      id: 'footer-rag-link'
    },
    {
      label: 'Agent Creator',
      href: '/agent-creator',
      id: 'footer-agent-link'
    }
  ],
  footerLinks: {
    project: {
      name: 'Astro X',
      url: 'https://github.com/MassivDash/astroX'
    },
    author: {
      name: 'Spaceghost',
      url: 'https://github.com/MassivDash'
    },
    website: {
      name: 'spaceout.pl',
      url: 'https://spaceout.pl'
    }
  }
}

