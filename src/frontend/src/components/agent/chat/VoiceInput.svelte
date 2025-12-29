<script lang="ts">
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import { useSpeechRecognition } from '../../../hooks/useSpeechRecognition.svelte.ts'

  interface Props {
    loading?: boolean
    onTranscript: (_text: string) => void
    onCommand: (_command: string) => void
    ttsEnabled?: boolean
    onToggleTTS?: () => void
    ttsSpeaking?: boolean
    onStopTTS?: () => void
  }

  let {
    loading = false,
    onTranscript,
    onCommand,
    ttsEnabled = false,
    onToggleTTS,
    ttsSpeaking = false,
    onStopTTS
  }: Props = $props()

  let alwaysOn = $state(false)
  let silenceTimer: any = $state(null)

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
        // If TTS is enabled, we wait for speaking to finish (handled by effect)
        // If TTS is disabled, we just restart with delay
        if (!ttsEnabled) {
          setTimeout(() => {
            speech.start()
          }, 200)
        }
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
    }
  })

  // Watch for TTS speaking state changes
  $effect(() => {
    if (ttsSpeaking) {
      // Stop listening when speaking starts
      if (speech.isListening) {
        speech.stop()
      }
      stopSilenceTimer()
    } else {
      // Restart listening when speaking ends IF alwaysOn is active
      // Only restart if we are not currently listening and alwaysOn is true
      if (alwaysOn && !speech.isListening && !loading) {
        setTimeout(() => {
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
      <button
        type="button"
        class="voice-input-button"
        class:listening={speech.isListening}
        class:error={!!speech.error}
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
      </button>

      <button
        type="button"
        class="voice-input-button"
        class:listening={alwaysOn}
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
      </button>

      {#if onToggleTTS}
        <button
          type="button"
          class="voice-input-button"
          class:speaking={ttsEnabled || ttsSpeaking}
          onclick={() => {
            if (ttsSpeaking && onStopTTS) {
              onStopTTS()
            } else if (onToggleTTS) {
              onToggleTTS()
            }
          }}
          title={ttsSpeaking
            ? 'Stop Speaking'
            : ttsEnabled
              ? 'Read Messages: On'
              : 'Read Messages: Off'}
        >
          <MaterialIcon
            name={ttsSpeaking
              ? 'stop'
              : ttsEnabled
                ? 'volume-high'
                : 'volume-off'}
            width="16"
            height="16"
          />
          <span class="label">
            {ttsSpeaking
              ? 'Speaking'
              : ttsEnabled
                ? 'Read Messages: On'
                : 'Read Messages: Off'}
          </span>
        </button>
      {/if}
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
    width: 100%;
    gap: 0.5rem;
    margin-bottom: 2rem;
    margin-left: 1rem;
  }

  .voice-input-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border: none;
    background: transparent;
    border-radius: 8px;
    cursor: pointer;
    color: var(--text-secondary, #666);
    transition: all 0.2s ease;
    font-size: 0.9rem;
    justify-content: flex-start;
    min-width: 10rem;
  }

  .voice-input-button:active {
    color: var(--accent-color, #2196f3);
    background-color: rgba(33, 150, 243, 0.1);
  }

  .voice-input-button.speaking {
    color: var(--accent-color, #2196f3);
    background-color: rgba(33, 150, 243, 0.1);
  }

  .voice-input-button:hover:not(:disabled) {
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--accent-color, #2196f3);
  }

  .voice-input-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .voice-input-button.listening {
    color: #f44336;
    background-color: rgba(244, 67, 54, 0.1);
    animation: pulse 2s infinite;
  }

  .voice-input-button.error {
    color: #ff9800;
  }

  .voice-input {
    position: relative;
    display: block;
    width: 100%;
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
