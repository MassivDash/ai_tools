import { describe, expect, it } from 'vitest'
import astroConfig from './astro.config.mjs'

describe('astro.config', () => {
  it('does not include @astrojs/react integration to avoid deprecated vite rolldown warnings', () => {
    const integrationNames = (astroConfig.integrations || []).map(
      (integration: { name?: string }) => integration?.name
    )
    expect(integrationNames).not.toContain('@astrojs/react')
  })

  it('includes @astrojs/svelte integration', () => {
    const integrationNames = (astroConfig.integrations || []).map(
      (integration: { name?: string }) => integration?.name
    )
    expect(integrationNames).toContain('@astrojs/svelte')
  })

  it('configures chunkSizeWarningLimit to avoid chunk size warnings', () => {
    expect(
      astroConfig.vite?.build?.chunkSizeWarningLimit
    ).toBeGreaterThanOrEqual(3500)
  })
})
