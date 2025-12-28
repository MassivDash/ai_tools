<script lang="ts">
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import { useSpeechRecognition } from '../../../hooks/useSpeechRecognition.svelte.ts'

  interface Props {
    loading?: boolean
    onTranscript: (text: string) => void
    onCommand: (command: 'execute' | 'send') => void
  }

  let { loading = false, onTranscript, onCommand }: Props = $props()

  const speech = useSpeechRecognition({
    onTranscript: (text) => {
      onTranscript(text)
    },
    onCommand: (command) => {
      onCommand(command)
    },
    onError: (err) => {
      console.error('Speech error', err)
    }
  })
</script>

<div class="voice-input">
  <button
    type="button"
    class="file-button voice-button"
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
      width="20"
      height="20"
    />
  </button>
  {#if speech.error}
    <div class="error-tooltip">{speech.error}</div>
  {/if}
</div>

<style>
  .file-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: none;
    background: transparent;
    border-radius: 8px;
    cursor: pointer;
    color: var(--text-secondary, #666);
    transition: all 0.2s ease;
    padding: 0;
  }

  .file-button:hover:not(:disabled) {
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--accent-color, #2196f3);
  }

  .file-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .voice-button.listening {
    color: #f44336;
    animation: pulse 1.5s infinite;
  }

  .voice-button.error {
    color: #ff9800;
  }

  .voice-input {
    position: relative;
    display: inline-block;
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
      transform: scale(1);
      box-shadow: 0 0 0 0 rgba(244, 67, 54, 0.7);
    }
    70% {
      transform: scale(1.1);
      box-shadow: 0 0 0 5px rgba(244, 67, 54, 0);
    }
    100% {
      transform: scale(1);
      box-shadow: 0 0 0 0 rgba(244, 67, 54, 0);
    }
  }
</style>
