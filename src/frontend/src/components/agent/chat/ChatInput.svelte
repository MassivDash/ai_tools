<script lang="ts">
  import Button from '../../ui/Button.svelte'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ModelCapabilities } from '../types'

  interface Props {
    inputMessage?: string
    loading?: boolean
    modelCapabilities?: ModelCapabilities
    onSend: () => void
    onInputChange: (value: string) => void
  }

  let {
    inputMessage = $bindable(''),
    loading = false,
    modelCapabilities = { vision: false, audio: false },
    onSend,
    onInputChange
  }: Props = $props()

  let textareaElement: HTMLTextAreaElement = $state()
  let fileInputRef: HTMLInputElement = $state()
  let audioInputRef: HTMLInputElement = $state()
  let imageInputRef: HTMLInputElement = $state()
  let pdfInputRef: HTMLInputElement = $state()

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      onSend()
    }
  }

  const autoResize = () => {
    if (textareaElement) {
      textareaElement.style.height = 'auto'
      textareaElement.style.height = `${Math.min(textareaElement.scrollHeight, 300)}px`
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

  const handleFileSelect = async (
    e: Event,
    type: 'text' | 'audio' | 'image' | 'pdf'
  ) => {
    const target = e.target as HTMLInputElement
    const file = target.files?.[0]
    if (!file) return

    try {
      let content = ''

      if (type === 'text') {
        // Handle text, md, txt files
        const fileContent = await file.text()
        content = `\n\n[File: ${file.name}]\n${fileContent}\n\n`
        console.log('📄 Text file processed:', file.name)
      } else if (type === 'pdf') {
        // Convert PDF to markdown using backend endpoint
        console.log('📄 Converting PDF to text:', file.name)
        const formData = new FormData()
        formData.append('file', file)
        formData.append('count_tokens', 'false')

        try {
          const response = await axiosBackendInstance.post<{
            markdown: string
            filename: string
          }>('pdf-to-markdown', formData, {
            headers: {
              'Content-Type': 'multipart/form-data'
            }
          })

          content = `\n\n[PDF: ${response.data.filename}]\n${response.data.markdown}\n\n`
          console.log('✅ PDF converted to text:', file.name)
        } catch (err: any) {
          console.error('❌ Failed to convert PDF:', err)
          content = `\n\n[PDF File: ${file.name} - Failed to extract text: ${err.response?.data?.error || err.message}]\n\n`
        }
      } else if (type === 'audio') {
        // For audio, encode as base64 for now
        // In the future, this could be sent as a separate attachment
        const reader = new FileReader()
        reader.onload = (event) => {
          const base64 = (event.target?.result as string)?.split(',')[1]
          const audioContent = `\n\n[Audio File: ${file.name}]\n[Base64 data: ${base64?.substring(0, 100)}...]\n\n`
          onInputChange(inputMessage + audioContent)
        }
        reader.readAsDataURL(file)
        target.value = ''
        return // Early return for async FileReader
      } else if (type === 'image') {
        // For images, encode as base64
        const reader = new FileReader()
        reader.onload = (event) => {
          const base64 = event.target?.result as string
          // Include image in markdown format
          const imageContent = `\n\n![${file.name}](${base64})\n\n`
          onInputChange(inputMessage + imageContent)
        }
        reader.readAsDataURL(file)
        target.value = ''
        return // Early return for async FileReader
      }

      // Append content to input message
      onInputChange(inputMessage + content)

      // Reset file input
      target.value = ''
    } catch (err) {
      console.error(`❌ Failed to process ${type} file:`, err)
    }
  }

  const triggerFileInput = (type: 'text' | 'audio' | 'image' | 'pdf') => {
    if (type === 'text') {
      fileInputRef?.click()
    } else if (type === 'audio') {
      audioInputRef?.click()
    } else if (type === 'image') {
      imageInputRef?.click()
    } else if (type === 'pdf') {
      pdfInputRef?.click()
    }
  }
</script>

<div class="chat-input-container">
  <div class="input-wrapper">
    <div class="file-buttons">
      {#if modelCapabilities.audio}
        <button
          type="button"
          class="file-button"
          onclick={() => triggerFileInput('audio')}
          disabled={loading}
          title="Upload audio file"
        >
          <MaterialIcon name="microphone" width="20" height="20" />
        </button>
      {/if}
      {#if modelCapabilities.vision}
        <button
          type="button"
          class="file-button"
          onclick={() => triggerFileInput('image')}
          disabled={loading}
          title="Upload image file"
        >
          <MaterialIcon name="image" width="20" height="20" />
        </button>
      {/if}
      <button
        type="button"
        class="file-button"
        onclick={() => triggerFileInput('text')}
        disabled={loading}
        title="Upload text file (txt, md)"
      >
        <MaterialIcon name="note-text" width="20" height="20" />
      </button>
      <button
        type="button"
        class="file-button"
        onclick={() => triggerFileInput('pdf')}
        disabled={loading}
        title="Upload PDF file"
      >
        <MaterialIcon name="file-pdf-box" width="20" height="20" />
      </button>
    </div>

    <textarea
      bind:this={textareaElement}
      bind:value={inputMessage}
      onkeypress={handleKeyPress}
      oninput={handleInput}
      placeholder="Type your message... (Press Enter to send, Shift+Enter for new line)"
      disabled={loading}
      class="chat-input"
      rows="3"
    ></textarea>

    <div class="send-button-wrapper">
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

  <!-- Hidden file inputs -->
  <input
    bind:this={fileInputRef}
    type="file"
    accept=".txt,.md,.text"
    onchange={(e) => handleFileSelect(e, 'text')}
    style="display: none"
  />
  {#if modelCapabilities.audio}
    <input
      bind:this={audioInputRef}
      type="file"
      accept="audio/*"
      onchange={(e) => handleFileSelect(e, 'audio')}
      style="display: none"
    />
  {/if}
  {#if modelCapabilities.vision}
    <input
      bind:this={imageInputRef}
      type="file"
      accept="image/*"
      onchange={(e) => handleFileSelect(e, 'image')}
      style="display: none"
    />
  {/if}
  <input
    bind:this={pdfInputRef}
    type="file"
    accept=".pdf"
    onchange={(e) => handleFileSelect(e, 'pdf')}
    style="display: none"
  />
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
    padding: 1rem;
    transition: all 0.2s ease;
    min-height: 80px;
  }

  .input-wrapper:focus-within {
    border-color: var(--accent-color, #2196f3);
    box-shadow: 0 0 0 3px rgba(33, 150, 243, 0.1);
  }

  .file-buttons {
    display: flex;
    flex-direction: row;
    gap: 0.5rem;
    align-items: center;
    justify-content: flex-start;
    padding-right: 0.5rem;
    border-right: 1px solid var(--border-color, #e0e0e0);
  }

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

  .chat-input {
    flex: 1;
    padding: 0.75rem;
    border: none;
    background: transparent;
    font-family: inherit;
    font-size: 1rem;
    resize: none;
    min-height: 3rem;
    max-height: 300px;
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

  .send-button-wrapper {
    display: flex;
    align-items: flex-end;
    padding-left: 0.5rem;
  }

  .send-button {
    min-width: 44px;
    height: 44px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  @media screen and (max-width: 768px) {
    .chat-input-container {
      padding: 1rem;
    }

    .input-wrapper {
      padding: 0.75rem;
      min-height: 70px;
    }

    .file-buttons {
      gap: 0.5rem;
      padding-right: 0.5rem;
    }

    .file-button {
      width: 32px;
      height: 32px;
    }

    .chat-input {
      font-size: 0.9rem;
      padding: 0.5rem;
    }

    .send-button {
      min-width: 40px;
      height: 40px;
    }
  }
</style>
