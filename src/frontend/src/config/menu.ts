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
      label: 'Llama Server',
      href: '/',
      id: 'home-link',
      icon: 'server-network'
    },
    {
      label: 'Agent Chat',
      href: '/agent',
      id: 'footer-agent-link',
      icon: 'robot'
    },
    {
      label: 'Vector Database',
      href: '/database',
      id: 'database-link',
      icon: 'database'
    },
    {
      label: 'Model Notes',
      href: '/model-notes',
      id: 'model-notes-link',
      icon: 'note'
    },
    {
      label: 'Tools',
      href: '/tools',
      id: 'tools-link',
      icon: 'wrench'
    }
  ],
  footer: [
    {
      label: 'Llama Server',
      href: '/',
      id: 'footer-main-link'
    },
    {
      label: 'Agent Chat',
      href: '/agent-creator',
      id: 'footer-agent-link'
    },
    {
      label: 'Vector Database',
      href: '/database',
      id: 'footer-rag-link'
    },
    {
      label: 'Tools',
      href: '/tools',
      id: 'footer-tools-link'
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
