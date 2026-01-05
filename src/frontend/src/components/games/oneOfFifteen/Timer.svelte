<script lang="ts">
  import { onDestroy } from 'svelte'

  interface Props {
    startTime: number | undefined
    duration?: number
    onTimeout?: () => void
  }

  let { startTime, duration = 60, onTimeout }: Props = $props()

  let timeLeft = $state(0)
  let timerInterval: any

  $effect(() => {
    if (startTime) {
      clearInterval(timerInterval)
      timerInterval = setInterval(() => {
        const now = Date.now() / 1000
        const start = startTime || 0
        const elapsed = now - start
        timeLeft = Math.max(0, duration - Math.floor(elapsed))

        if (timeLeft === 0) {
          clearInterval(timerInterval)
          if (onTimeout) onTimeout()
        }
      }, 1000)
    } else {
      timeLeft = duration // Reset if no start time
    }
  })

  onDestroy(() => {
    if (timerInterval) clearInterval(timerInterval)
  })
</script>

<div class="timer" class:critical={timeLeft <= 10}>
  <div class="timer-value">{timeLeft}</div>
  <div class="timer-label">seconds</div>
</div>

<style>
  .timer {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 50%;
    width: 100px;
    height: 100px;
    border: 4px solid var(--primary-color);
    transition: all 0.3s ease;
  }

  .timer.critical {
    border-color: var(--error);
    animation: pulse 1s infinite;
  }

  .timer-value {
    font-size: 2.5rem;
    font-weight: 800;
    line-height: 1;
  }

  .timer-label {
    font-size: 0.8rem;
    text-transform: uppercase;
    opacity: 0.8;
  }

  @keyframes pulse {
    0% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.05);
    }
    100% {
      transform: scale(1);
    }
  }
</style>
