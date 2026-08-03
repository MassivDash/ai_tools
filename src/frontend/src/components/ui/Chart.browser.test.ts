/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import Chart from './Chart.svelte'

// jsdom has no canvas 2d context, so chart.js is replaced by a fake that records
// the config it was constructed with. The fake also resolves the handful of
// option defaults real chart.js fills in (scales.x/y, legend.labels), because
// Chart.svelte's theme pass writes into them.
const chartMock = vi.hoisted(() => {
  const instances: any[] = []

  class FakeChart {
    canvas: HTMLCanvasElement
    type: string
    data: any
    options: any
    update = vi.fn()
    destroy = vi.fn()

    constructor(canvas: HTMLCanvasElement, config: any) {
      this.canvas = canvas
      this.type = config.type
      this.data = config.data
      this.options = config.options
      this.options.scales = this.options.scales ?? {}
      this.options.scales.x = this.options.scales.x ?? {}
      this.options.scales.y = this.options.scales.y ?? {}
      this.options.plugins.legend.labels =
        this.options.plugins.legend.labels ?? {}
      instances.push(this)
    }
  }

  return { instances, FakeChart }
})

vi.mock('chart.js/auto', () => ({ default: chartMock.FakeChart }))

const { instances } = chartMock

const lastChart = () => instances[instances.length - 1]

const lineData = (overrides: Record<string, unknown> = {}) => ({
  type: 'line' as const,
  title: 'Revenue',
  xAxis: { label: 'Month', data: ['Jan', 'Feb', 'Mar'] },
  series: [{ name: 'Sales', data: [1, 2, 3] }],
  ...overrides
})

const renderChart = async (data: any) => {
  const result = render(Chart, { props: { data } })
  await waitFor(() => expect(instances.length).toBeGreaterThan(0))
  return result
}

beforeEach(() => {
  instances.length = 0
  document.documentElement.classList.remove('dark')
  document.documentElement.style.removeProperty('--md-primary')
})

afterEach(() => {
  document.documentElement.classList.remove('dark')
  document.documentElement.style.removeProperty('--md-primary')
  vi.restoreAllMocks()
})

test('renders a canvas in the container and hands it to chart.js', async () => {
  const { container } = await renderChart(lineData())

  const canvas = container.querySelector('.chart-container canvas')
  expect(canvas).toBeTruthy()
  expect(lastChart().canvas).toBe(canvas)
})

test('builds the chart config from the data prop', async () => {
  await renderChart(lineData())

  const chart = lastChart()
  expect(chart.type).toBe('line')
  expect(chart.data.labels).toEqual(['Jan', 'Feb', 'Mar'])
  expect(chart.data.datasets).toHaveLength(1)
  expect(chart.data.datasets[0]).toMatchObject({
    label: 'Sales',
    data: [1, 2, 3],
    borderWidth: 2,
    tension: 0.3
  })
  expect(chart.options.responsive).toBe(true)
  expect(chart.options.maintainAspectRatio).toBe(false)
  expect(chart.options.plugins.title).toMatchObject({
    display: true,
    text: 'Revenue',
    font: { size: 16, weight: 'bold' }
  })
  expect(chart.options.plugins.legend.position).toBe('top')
  expect(chart.options.plugins.tooltip).toMatchObject({
    mode: 'index',
    intersect: false
  })
  expect(chart.options.scales.y.beginAtZero).toBe(false)
})

test('hides the title plugin when no title is supplied', async () => {
  await renderChart(lineData({ title: undefined }))

  expect(lastChart().options.plugins.title.display).toBe(false)
  expect(lastChart().options.plugins.title.text).toBeUndefined()
})

test('passes through the bar chart type', async () => {
  await renderChart(lineData({ type: 'bar' }))

  expect(lastChart().type).toBe('bar')
})

test('a single series uses the --md-primary brand colour', async () => {
  document.documentElement.style.setProperty('--md-primary', '#ff0000')

  await renderChart(lineData())

  expect(lastChart().data.datasets[0].borderColor).toBe('#ff0000')
  expect(lastChart().data.datasets[0].backgroundColor).toBe('#ff000033')
})

test('a single series falls back to the default blue when the CSS var is unset', async () => {
  await renderChart(lineData())

  expect(lastChart().data.datasets[0].borderColor).toBe('#2196f3')
  expect(lastChart().data.datasets[0].backgroundColor).toBe('#2196f333')
})

test('multiple series take successive light palette colours', async () => {
  document.documentElement.style.setProperty('--md-primary', '#ff0000')

  await renderChart(
    lineData({
      series: [
        { name: 'a', data: [1] },
        { name: 'b', data: [2] },
        { name: 'c', data: [3] }
      ]
    })
  )

  const colors = lastChart().data.datasets.map((d: any) => d.borderColor)
  expect(colors).toEqual(['#2a78d6', '#eb6834', '#1baf7a'])
  expect(lastChart().data.datasets[1].backgroundColor).toBe('#eb683433')
})

test('an explicit series colour overrides the palette and survives theming', async () => {
  await renderChart(
    lineData({
      series: [
        { name: 'a', data: [1] },
        { name: 'b', data: [2], color: '#123456' }
      ]
    })
  )

  expect(lastChart().data.datasets[0].borderColor).toBe('#2a78d6')
  expect(lastChart().data.datasets[1].borderColor).toBe('#123456')
  expect(lastChart().data.datasets[1].backgroundColor).toBe('#12345633')
})

test('the palette wraps around after eight series', async () => {
  await renderChart(
    lineData({
      series: Array.from({ length: 9 }, (_, i) => ({
        name: `s${i}`,
        data: [i]
      }))
    })
  )

  const colors = lastChart().data.datasets.map((d: any) => d.borderColor)
  expect(colors[0]).toBe('#2a78d6')
  expect(colors[7]).toBe('#e34948')
  // 9th series cycles back to the first palette entry.
  expect(colors[8]).toBe('#2a78d6')
})

test('applies light theme colours to axes, legend and title', async () => {
  await renderChart(lineData())

  const chart = lastChart()
  expect(chart.options.scales.x.ticks.color).toBe('#666666')
  expect(chart.options.scales.x.grid).toEqual({ color: '#e0e0e0' })
  expect(chart.options.scales.y.ticks.color).toBe('#666666')
  expect(chart.options.scales.y.grid).toEqual({ color: '#e0e0e0' })
  expect(chart.options.plugins.legend.labels.color).toBe('#666666')
  expect(chart.options.plugins.title.color).toBe('#100f0f')
  // The theme pass runs once as part of chart creation.
  expect(chart.update).toHaveBeenCalledTimes(1)
})

test('applies dark theme colours and the dark palette when mounted in dark mode', async () => {
  document.documentElement.classList.add('dark')

  await renderChart(
    lineData({
      series: [
        { name: 'a', data: [1] },
        { name: 'b', data: [2] }
      ]
    })
  )

  const chart = lastChart()
  expect(chart.options.scales.x.ticks.color).toBe('#e0e0e0')
  expect(chart.options.scales.y.grid).toEqual({ color: '#404040' })
  expect(chart.options.plugins.legend.labels.color).toBe('#e0e0e0')
  expect(chart.options.plugins.title.color).toBe('#ffffff')
  expect(chart.data.datasets.map((d: any) => d.borderColor)).toEqual([
    '#3987e5',
    '#d95926'
  ])
})

test('re-themes the chart when the dark class is toggled on the root element', async () => {
  await renderChart(
    lineData({
      series: [
        { name: 'a', data: [1] },
        { name: 'b', data: [2] }
      ]
    })
  )

  const chart = lastChart()
  expect(chart.data.datasets.map((d: any) => d.borderColor)).toEqual([
    '#2a78d6',
    '#eb6834'
  ])

  document.documentElement.classList.add('dark')

  await waitFor(() => {
    expect(chart.update).toHaveBeenCalledTimes(2)
  })
  expect(chart.data.datasets.map((d: any) => d.borderColor)).toEqual([
    '#3987e5',
    '#d95926'
  ])
  expect(chart.options.plugins.title.color).toBe('#ffffff')

  document.documentElement.classList.remove('dark')

  await waitFor(() => {
    expect(chart.update).toHaveBeenCalledTimes(3)
  })
  expect(chart.data.datasets.map((d: any) => d.borderColor)).toEqual([
    '#2a78d6',
    '#eb6834'
  ])
})

test('ignores mutations of attributes other than class', async () => {
  await renderChart(lineData())
  const chart = lastChart()

  document.documentElement.setAttribute('data-unrelated', 'x')
  await new Promise((resolve) => setTimeout(resolve, 10))

  expect(chart.update).toHaveBeenCalledTimes(1)
  document.documentElement.removeAttribute('data-unrelated')
})

test('rebuilds the chart when the data prop changes', async () => {
  const { rerender } = await renderChart(lineData())

  const previous = lastChart()
  const countBefore = instances.length

  await rerender({
    data: {
      type: 'bar',
      xAxis: { label: 'Quarter', data: ['Q1', 'Q2'] },
      series: [{ name: 'Costs', data: [9, 8] }]
    }
  })

  expect(instances.length).toBe(countBefore + 1)
  expect(previous.destroy).toHaveBeenCalledTimes(1)
  const chart = lastChart()
  expect(chart.type).toBe('bar')
  expect(chart.data.labels).toEqual(['Q1', 'Q2'])
  expect(chart.data.datasets[0].label).toBe('Costs')
})

test('destroys the chart and disconnects the observer on unmount', async () => {
  const disconnect = vi.spyOn(MutationObserver.prototype, 'disconnect')

  const { unmount } = await renderChart(lineData())
  const chart = lastChart()
  expect(chart.destroy).not.toHaveBeenCalled()

  unmount()

  expect(chart.destroy).toHaveBeenCalledTimes(1)
  expect(disconnect).toHaveBeenCalled()

  // A later theme toggle no longer touches the destroyed chart.
  document.documentElement.classList.add('dark')
  await new Promise((resolve) => setTimeout(resolve, 10))
  expect(chart.update).toHaveBeenCalledTimes(1)
})
