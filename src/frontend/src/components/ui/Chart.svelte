<script lang="ts">
  import { onMount } from 'svelte'
  import Chart from 'chart.js/auto'

  interface ChartData {
    type: 'line' | 'bar'
    title?: string
    xAxis: {
      label: string
      data: string[]
    }
    series: Array<{
      name: string
      data: number[]
      color?: string
    }>
  }

  export let data: ChartData

  let canvas: HTMLCanvasElement
  let chartInstance: Chart | null = null
  let observer: MutationObserver | null = null

  // Helpers to get CSS variable values
  const getCssVar = (name: string, fallback: string) => {
    if (typeof getComputedStyle === 'undefined') return fallback
    const value = getComputedStyle(document.documentElement)
      .getPropertyValue(name)
      .trim()
    return value || fallback
  }

  const updateChartTheme = () => {
    if (!chartInstance) return

    const isDark = document.documentElement.classList.contains('dark')
    // Get current primary color from CSS (it changes based on class)
    const primaryColor = getCssVar('--md-primary', '#2196f3')

    const textColor = isDark ? '#e0e0e0' : '#666666'
    const gridColor = isDark ? '#404040' : '#e0e0e0'
    const titleColor = isDark ? '#ffffff' : '#100f0f'

    // Update scales
    if (chartInstance.options.scales?.x) {
      chartInstance.options.scales.x.ticks = {
        ...chartInstance.options.scales.x.ticks,
        color: textColor
      }
      chartInstance.options.scales.x.grid = {
        color: gridColor
      }
    }

    if (chartInstance.options.scales?.y) {
      chartInstance.options.scales.y.ticks = {
        ...chartInstance.options.scales.y.ticks,
        color: textColor
      }
      chartInstance.options.scales.y.grid = {
        color: gridColor
      }
    }

    // Update legends and title
    if (chartInstance.options.plugins?.legend) {
      chartInstance.options.plugins.legend.labels.color = textColor
    }

    if (chartInstance.options.plugins?.title) {
      chartInstance.options.plugins.title.color = titleColor
    }

    // Update datasets colors
    chartInstance.data.datasets.forEach((dataset, i) => {
      // Only update if the original data didn't specify a color
      const originalSeries = data.series[i]
      if (!originalSeries?.color) {
        dataset.borderColor = primaryColor
        dataset.backgroundColor = primaryColor + '33'
      }
    })

    chartInstance.update()
  }

  const createChart = () => {
    if (chartInstance) chartInstance.destroy()
    if (!canvas) return

    // Initial color fetch
    const primaryColor = getCssVar('--md-primary', '#2196f3')

    const datasets = data.series.map((s) => {
      // Use series color if provided, otherwise use primary color
      const color = s.color || primaryColor

      return {
        label: s.name,
        data: s.data,
        borderColor: color,
        backgroundColor: color + '33', // Add transparency for fill
        borderWidth: 2,
        tension: 0.3
      }
    })

    chartInstance = new Chart(canvas, {
      type: data.type,
      data: {
        labels: data.xAxis.data,
        datasets: datasets
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          title: {
            display: !!data.title,
            text: data.title,
            font: {
              size: 16,
              weight: 'bold'
            }
          },
          legend: {
            position: 'top'
          },
          tooltip: {
            mode: 'index',
            intersect: false
          }
        },
        scales: {
          y: {
            beginAtZero: false
          }
        }
      }
    })

    updateChartTheme()
  }

  onMount(() => {
    createChart()

    // Watch for class changes on the html element (dark mode toggle)
    observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (
          mutation.type === 'attributes' &&
          mutation.attributeName === 'class'
        ) {
          updateChartTheme()
        }
      }
    })

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class']
    })

    return () => {
      if (chartInstance) chartInstance.destroy()
      if (observer) observer.disconnect()
    }
  })

  // React to data changes
  $: if (data && canvas) {
    createChart()
  }
</script>

<div class="chart-container">
  <canvas bind:this={canvas}></canvas>
</div>

<style>
  .chart-container {
    background: var(--bg-primary, #fff);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    padding: 1rem;
    margin: 1rem 0;
    position: relative;
    width: 100%;
    min-height: 350px; /* Slightly taller for legend */
  }

  /* Dark mode support logic is handled in JS via Chart.js options, 
     but container background relies on CSS vars */
  @media (prefers-color-scheme: dark) {
    .chart-container {
      background: var(--bg-primary, #1e1e1e);
      border-color: var(--border-color, #404040);
    }
  }
</style>
