<script lang="ts">
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  import Button from '@ui/Button.svelte'
  import { useSpeechRecognition } from '@hooks/useSpeechRecognition.svelte.ts'

  interface Props {
    loading?: boolean
    onTranscript: (_text: string) => void
    onCommand: (_command: string) => void
    ttsSpeaking?: boolean
    lang?: string
  }

  let {
    loading = false,
    onTranscript,
    onCommand,
    ttsSpeaking = false,
    lang = 'en-US'
  }: Props = $props()

  let alwaysOn = $state(false)
  let silenceTimer: any = $state(null)
  let restartTimer: any = $state(null)

  const stopSilenceTimer = () => {
    if (silenceTimer) {
      clearTimeout(silenceTimer)
      silenceTimer = null
    }
  }

  const startSilenceTimer = () => {
    stopSilenceTimer()
    if (alwaysOn && speech.isListening) {
      silenceTimer = setTimeout(() => {
        // Auto-send after 2 seconds of silence
        if (speech.isListening) {
          speech.stop()
          onCommand('send')
        }
      }, 2000)
    }
  }

  const speech = useSpeechRecognition({
    onTranscript: (text) => {
      onTranscript(text)
    },
    onCommand: (command) => {
      onCommand(command)
      stopSilenceTimer()

      if (alwaysOn) {
        // If TTS is disabled (implicit here as we don't control it directly in this logic block),
        // we just restart with delay. Note: The parent component handles preventing restart if TTS starts speaking via the effect below
        restartTimer = setTimeout(() => {
          if (!ttsSpeaking) {
            speech.start()
          }
        }, 200)
      }
    },
    onError: (err) => {
      console.error('Speech error', err)
      stopSilenceTimer()
    },
    onEvent: (type) => {
      if (type === 'result') {
        startSilenceTimer()
      } else if (type === 'end' || type === 'error') {
        stopSilenceTimer()
      }
    },
    lang: () => lang
  })

  // Watch for TTS speaking state changes
  $effect(() => {
    if (ttsSpeaking) {
      // Stop listening when speaking starts
      if (speech.isListening) {
        speech.stop()
      }
      stopSilenceTimer()
      // Also clear any pending restart timer
      if (restartTimer) {
        clearTimeout(restartTimer)
        restartTimer = null
      }
    } else {
      // Restart listening when speaking ends IF alwaysOn is active
      // Only restart if we are not currently listening and alwaysOn is true
      if (alwaysOn && !speech.isListening && !loading) {
        restartTimer = setTimeout(() => {
          speech.start()
        }, 200)
      }
    }
  })

  // Clean up timer on destroy or when alwaysOn changes
  $effect(() => {
    if (!alwaysOn) {
      stopSilenceTimer()
    }
  })

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.code === 'Space') {
      const activeElement = document.activeElement as HTMLElement
      const isInput =
        activeElement.tagName === 'INPUT' ||
        activeElement.tagName === 'TEXTAREA' ||
        activeElement.isContentEditable

      if (!isInput && speech.isSupported) {
        e.preventDefault() // Prevent scrolling
        speech.toggle()
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if speech.isSupported}
  <div class="voice-input">
    <div class="voice-controls">
      <Button
        variant="ghost"
        class="voice-input-button {speech.isListening
          ? 'listening'
          : ''} {speech.error ? 'error' : ''}"
        onclick={speech.toggle}
        disabled={loading}
        title={speech.error ||
          (speech.isListening ? 'Stop Listening' : 'Start Voice Input')}
      >
        <MaterialIcon
          name={speech.error
            ? 'alert-circle'
            : speech.isListening
              ? 'microphone-off'
              : 'microphone'}
          width="18"
          height="18"
        />
        <span class="label">
          {speech.error
            ? 'Error'
            : speech.isListening
              ? "Say 'execute' or 2s pause"
              : 'Talk to agent'}
        </span>
      </Button>

      <Button
        variant="ghost"
        class="voice-input-button {alwaysOn ? 'listening' : ''}"
        onclick={() => (alwaysOn = !alwaysOn)}
        title="Always On: Auto-restart after sending"
      >
        <MaterialIcon
          name={alwaysOn ? 'repeat' : 'repeat-off'}
          width="16"
          height="16"
        />
        <span class="label">
          {alwaysOn ? 'Conversation mode on' : 'Conversation mode off'}
        </span>
      </Button>
    </div>
    {#if speech.error}
      <div class="error-tooltip">{speech.error}</div>
    {/if}
  </div>
{/if}

<style>
  .voice-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  /* ... skipping ... */
  .voice-input {
    position: relative;
    display: block;
  }

  .label {
    line-height: 1;
    font-weight: 500;
  }

  .error-tooltip {
    position: absolute;
    bottom: 100%;
    left: 50%;
    transform: translateX(-50%);
    background-color: #333;
    color: white;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
    white-space: nowrap;
    margin-bottom: 8px;
    z-index: 10;
  }

  @keyframes pulse {
    0% {
      box-shadow: 0 0 0 0 rgba(244, 67, 54, 0.4);
    }
    70% {
      box-shadow: 0 0 0 6px rgba(244, 67, 54, 0);
    }
    100% {
      box-shadow: 0 0 0 0 rgba(244, 67, 54, 0);
    }
  }
</style>
