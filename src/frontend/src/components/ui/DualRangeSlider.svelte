<script lang="ts">
  interface Props {
    min?: number
    max?: number
    step?: number
    minValue?: number
    maxValue?: number
    tickInterval?: number
  }

  let {
    min = 0,
    max = 100,
    step = 1,
    minValue = $bindable(0),
    maxValue = $bindable(100),
    tickInterval = 4
  }: Props = $props()

  let sliderContainer: HTMLDivElement
  let isDragging: 'min' | 'max' | null = $state(null)
  let startX = 0
  let startMinValue = 0
  let startMaxValue = 0

  // Generate tick marks (reactive to prop changes)
  const tickMarks = $derived(
    Array.from(
      { length: Math.floor((max - min) / tickInterval) + 1 },
      (_, i) => min + i * tickInterval
    )
  )

  // Calculate percentage position
  const getPercentage = (value: number): number => {
    return ((value - min) / (max - min)) * 100
  }

  // Get value from percentage
  const getValueFromPercentage = (percentage: number): number => {
    const rawValue = min + (percentage / 100) * (max - min)
    return Math.round(rawValue / step) * step
  }

  // Clamp value to range
  const clamp = (value: number): number => {
    return Math.max(min, Math.min(max, value))
  }

  // Handle mouse/touch start
  const handleStart = (e: MouseEvent | TouchEvent, thumb: 'min' | 'max') => {
    e.preventDefault()
    isDragging = thumb
    startX = 'touches' in e ? e.touches[0].clientX : e.clientX
    startMinValue = minValue
    startMaxValue = maxValue

    const handleMove = (moveEvent: MouseEvent | TouchEvent) => {
      if (!isDragging || !sliderContainer) return

      const currentX =
        'touches' in moveEvent
          ? moveEvent.touches[0].clientX
          : moveEvent.clientX
      const rect = sliderContainer.getBoundingClientRect()
      const percentage = ((currentX - rect.left) / rect.width) * 100
      const newValue = clamp(getValueFromPercentage(percentage))

      if (isDragging === 'min') {
        minValue = Math.min(newValue, maxValue)
      } else {
        maxValue = Math.max(newValue, minValue)
      }
    }

    const handleEnd = () => {
      isDragging = null
      document.removeEventListener('mousemove', handleMove as EventListener)
      document.removeEventListener('mouseup', handleEnd)
      document.removeEventListener('touchmove', handleMove as EventListener)
      document.removeEventListener('touchend', handleEnd)
    }

    document.addEventListener('mousemove', handleMove as EventListener)
    document.addEventListener('mouseup', handleEnd)
    document.addEventListener('touchmove', handleMove as EventListener)
    document.addEventListener('touchend', handleEnd)
  }

  // Ensure minValue doesn't exceed maxValue and vice versa
  $effect(() => {
    if (minValue > maxValue) {
      minValue = maxValue
    }
    if (maxValue < minValue) {
      maxValue = minValue
    }
  })
</script>

<div class="dual-range-slider" bind:this={sliderContainer}>
  <div class="slider-track">
    <div
      class="slider-fill"
      style="left: {getPercentage(minValue)}%; width: {getPercentage(maxValue) -
        getPercentage(minValue)}%"
    ></div>
  </div>

  <div class="slider-thumbs">
    <button
      class="thumb thumb-min"
      class:dragging={isDragging === 'min'}
      style="left: {getPercentage(minValue)}%"
      onmousedown={(e) => handleStart(e, 'min')}
      ontouchstart={(e) => handleStart(e, 'min')}
      aria-label="Minimum value"
      type="button"
    >
      <span class="thumb-value">{minValue}</span>
    </button>
    <button
      class="thumb thumb-max"
      class:dragging={isDragging === 'max'}
      style="left: {getPercentage(maxValue)}%"
      onmousedown={(e) => handleStart(e, 'max')}
      ontouchstart={(e) => handleStart(e, 'max')}
      aria-label="Maximum value"
      type="button"
    >
      <span class="thumb-value">{maxValue}</span>
    </button>
  </div>

  <div class="slider-ticks">
    {#each tickMarks as tick}
      <div class="tick" style="left: {getPercentage(tick)}%">
        <span class="tick-line"></span>
        <span class="tick-label">{tick}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .dual-range-slider {
    position: relative;
    width: 100%;
    padding: 2rem 0 0.5rem 0;
    min-height: 50px;
    user-select: none;
  }

  .slider-track {
    position: absolute;
    top: calc(50% + 1rem);
    left: 0;
    right: 0;
    transform: translateY(-50%);
    height: 8px;
    background: var(--border-color, #ddd);
    border-radius: 8px;
  }

  .slider-fill {
    position: absolute;
    top: 0;
    height: 100%;
    background: var(--accent-color, #b12424);
    border-radius: 8px;
    transition:
      left 0.1s ease,
      width 0.1s ease;
    pointer-events: none;
  }

  .slider-thumbs {
    position: absolute;
    top: calc(50% + 1rem);
    left: 0;
    right: 0;
    transform: translateY(-50%);
  }

  .thumb {
    position: absolute;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--accent-color, #b12424);
    border: 2px solid var(--bg-primary, #fff);
    cursor: grab;
    transform: translate(-50%, -50%);
    top: 50%;
    margin-top: 0;
    box-shadow:
      0 2px 4px rgba(0, 0, 0, 0.2),
      0 0 0 2px var(--bg-primary, #fff);
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      transform 0.1s ease,
      box-shadow 0.1s ease;
    z-index: 3;
  }

  .thumb:active,
  .thumb.dragging {
    cursor: grabbing;
    transform: translate(-50%, -50%) scale(1.1);
    box-shadow:
      0 4px 8px rgba(0, 0, 0, 0.3),
      0 0 0 3px var(--bg-primary, #fff);
    z-index: 5;
  }

  .thumb-value {
    position: absolute;
    top: -30px;
    font-size: 0.75rem;
    color: var(--text-primary, #100f0f);
    background: var(--bg-primary, #fff);
    padding: 2px 6px;
    border-radius: 8px;
    border: 1px solid var(--border-color, #ddd);
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .thumb:hover .thumb-value,
  .thumb.dragging .thumb-value {
    opacity: 1;
  }

  .slider-ticks {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 20px;
    pointer-events: none;
  }

  .tick {
    position: absolute;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .tick-line {
    width: 1px;
    height: 10px;
    background: var(--border-color, #ddd);
  }

  .tick-label {
    font-size: 0.7rem;
    color: var(--text-secondary, #666);
    margin-top: 2px;
    white-space: nowrap;
  }
</style>
