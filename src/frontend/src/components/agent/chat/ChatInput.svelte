<script lang="ts">
  import Button from '../../ui/Button.svelte'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'

  interface Props {
    inputMessage?: string
    loading?: boolean
    onSend: () => void
    onInputChange: (value: string) => void
  }

  let {
    inputMessage = $bindable(''),
    loading = false,
    onSend,
    onInputChange
  }: Props = $props()

  let textareaElement: HTMLTextAreaElement

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      onSend()
    }
  }

  const autoResize = () => {
    if (textareaElement) {
      textareaElement.style.height = 'auto'
      textareaElement.style.height = `${Math.min(textareaElement.scrollHeight, 192)}px`
    }
  }

  $effect(() => {
    if (inputMessage) {
      autoResize()
    }
  })

  const handleInput = (e: Event) => {
    const target = e.target as HTMLTextAreaElement
    onInputChange(target.value)
    autoResize()
  }
</script>

<div class="chat-input-container">
  <div class="input-wrapper">
    <textarea
      bind:this={textareaElement}
      bind:value={inputMessage}
      onkeypress={handleKeyPress}
      oninput={handleInput}
      placeholder="Type your message... (Press Enter to send, Shift+Enter for new line)"
      disabled={loading}
      class="chat-input"
      rows="1"
    ></textarea>
    <Button
      variant="primary"
      onclick={onSend}
      disabled={loading || !inputMessage.trim()}
      class="send-button"
    >
      <MaterialIcon name="send" width="20" height="20" />
    </Button>
  </div>
</div>

<style>
  .chat-input-container {
    padding: 1.5rem;
    border-top: 1px solid var(--border-color, #e0e0e0);
    background-color: var(--bg-secondary, #f9f9f9);
  }

  .input-wrapper {
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
    max-width: 100%;
    margin: 0 auto;
    background-color: var(--bg-primary, #fff);
    border: 2px solid var(--border-color, #e0e0e0);
    border-radius: 24px;
    padding: 0.75rem 1rem;
    transition: all 0.2s ease;
  }

  .input-wrapper:focus-within {
    border-color: var(--accent-color, #2196f3);
    box-shadow: 0 0 0 3px rgba(33, 150, 243, 0.1);
  }

  .chat-input {
    flex: 1;
    padding: 0.5rem 0;
    border: none;
    background: transparent;
    font-family: inherit;
    font-size: 1rem;
    resize: none;
    min-height: 1.5rem;
    max-height: 8rem;
    line-height: 1.5;
    overflow-y: auto;
    color: var(--text-primary, #100f0f);
  }

  .chat-input:focus {
    outline: none;
  }

  .chat-input::placeholder {
    color: var(--text-tertiary, #bbb);
  }

  @media screen and (max-width: 768px) {
    .chat-input-container {
      padding: 1rem;
    }

    .input-wrapper {
      padding: 0.5rem 0.75rem;
    }
  }
</style>

