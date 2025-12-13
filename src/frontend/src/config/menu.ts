export interface MenuItem {
  label: string
  href: string
  external?: boolean
  id?: string
  icon?: string
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
      id: 'home-link',
      icon: 'codicon:copilot'
    },
    {
      label: 'Tools',
      href: '/tools',
      id: 'tools-link',
      icon: 'codicon:tools'
    },
    {
      label: 'Vector Database',
      href: '/database',
      id: 'database-link',
      icon: 'codicon:database'
    },
    {
      label: 'Agent Creator',
      href: '/agent-creator',
      id: 'footer-agent-link',
      icon: 'codicon:agent'
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
      href: '/database',
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

