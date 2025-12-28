<script lang="ts">
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import { useSpeechRecognition } from '../../../hooks/useSpeechRecognition.svelte.ts'

  interface Props {
    loading?: boolean
    onTranscript: (text: string) => void
    onCommand: (command: 'execute' | 'send') => void
  }

  let { loading = false, onTranscript, onCommand }: Props = $props()

  let alwaysOn = $state(false)

  const speech = useSpeechRecognition({
    onTranscript: (text) => {
      onTranscript(text)
    },
    onCommand: (command) => {
      onCommand(command)
      if (alwaysOn) {
        // Little delay to allow the stop to complete and buffer to clear
        setTimeout(() => {
          speech.start()
        }, 200)
      }
    },
    onError: (err) => {
      console.error('Speech error', err)
    }
  })
</script>

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
              ? "Say 'execute' when finished"
              : 'Dictate'}
        </span>
      </button>

      <button
        type="button"
        class="always-on-toggle"
        class:active={alwaysOn}
        onclick={() => (alwaysOn = !alwaysOn)}
        title="Always On: Auto-restart after sending"
      >
        <MaterialIcon
          name={alwaysOn ? 'repeat' : 'repeat-off'}
          width="16"
          height="16"
        />
      </button>
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
    flex: 1;
    justify-content: flex-start;
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

  .always-on-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    border-radius: 8px;
    cursor: pointer;
    color: var(--text-tertiary, #999);
    transition: all 0.2s ease;
  }

  .always-on-toggle:hover {
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--text-primary, #333);
  }

  .always-on-toggle.active {
    color: var(--accent-color, #2196f3);
    background-color: rgba(33, 150, 243, 0.1);
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
