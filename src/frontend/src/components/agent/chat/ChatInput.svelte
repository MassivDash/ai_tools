<script lang="ts">
  import Button from '../../ui/Button.svelte'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ModelCapabilities, FileAttachment } from '../types'

  interface Props {
    inputMessage?: string
    loading?: boolean
    modelCapabilities?: ModelCapabilities
    onSend: () => void
    onInputChange: (_value: string) => void
    onAttachmentsChange?: (_attachments: FileAttachment[]) => void
  }

  let {
    inputMessage = $bindable(''),
    loading = false,
    modelCapabilities = { vision: false, audio: false },
    onSend,
    onInputChange,
    onAttachmentsChange
  }: Props = $props()

  let textareaElement: HTMLTextAreaElement = $state()
  let fileInputRef: HTMLInputElement = $state()
  let audioInputRef: HTMLInputElement = $state()
  let imageInputRef: HTMLInputElement = $state()
  let pdfInputRef: HTMLInputElement = $state()

  // Track file attachments separately
  let attachments: FileAttachment[] = $state([])

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      // Clean input message before sending (remove any attachment references)
      const cleanedInput = cleanInputMessage(inputMessage)
      if (cleanedInput !== inputMessage) {
        onInputChange(cleanedInput)
      }
      onSend()
      clearAttachments()
    }
  }

  const autoResize = () => {
    if (textareaElement) {
      textareaElement.style.height = 'auto'
      textareaElement.style.height = `${Math.min(textareaElement.scrollHeight, 150)}px`
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

  const removeAttachment = (index: number) => {
    attachments = attachments.filter((_, i) => i !== index)
    onAttachmentsChange?.(attachments)
  }

  const clearAttachments = () => {
    attachments = []
    onAttachmentsChange?.(attachments)
  }

  // Clean input message from any attachment references
  const cleanInputMessage = (text: string): string => {
    // Remove patterns like [text:filename] or [pdf:filename]
    // Only remove the pattern, preserve spaces and other content
    return text.replace(/\[\w+:[^\]]+\]/g, '')
  }

  const handleFileSelect = async (
    e: Event,
    type: 'text' | 'audio' | 'image' | 'pdf'
  ) => {
    const target = e.target as HTMLInputElement
    const file = target.files?.[0]
    if (!file) return

    // Auto-detect type based on mime type to be robust
    let fileType = type
    if (file.type.startsWith('image/')) {
      fileType = 'image'
    } else if (file.type === 'application/pdf') {
      fileType = 'pdf'
    } else if (file.type.startsWith('audio/')) {
      fileType = 'audio'
    }

    try {
      const attachment: FileAttachment = {
        name: file.name,
        type: fileType,
        size: file.size
      }

      if (fileType === 'text') {
        // Handle text, md, txt files
        const fileContent = await file.text()
        attachment.content = fileContent
        // eslint-disable-next-line no-console
        console.log('📄 Text file processed:', file.name)
      } else if (fileType === 'pdf') {
        // Convert PDF to markdown using backend endpoint
        // eslint-disable-next-line no-console
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

          attachment.content = response.data.markdown
          // eslint-disable-next-line no-console
          console.log('✅ PDF converted to text:', file.name)
        } catch (err: any) {
          console.error('❌ Failed to convert PDF:', err)
          attachment.content = `Failed to extract text: ${err.response?.data?.error || err.message}`
        }
      } else if (fileType === 'audio') {
        // For audio, encode as base64 for now
        const reader = new FileReader()
        reader.onload = (event) => {
          const base64 = (event.target?.result as string)?.split(',')[1]
          attachment.content = base64 || ''
          attachments = [...attachments, attachment]
          // Clean input message to remove any attachment references
          const cleanedInput = cleanInputMessage(inputMessage)
          if (cleanedInput !== inputMessage) {
            onInputChange(cleanedInput)
          }
          onAttachmentsChange?.(attachments)
        }
        reader.readAsDataURL(file)
        target.value = ''
        return // Early return for async FileReader
      } else if (fileType === 'image') {
        // Process image: Resize if needed and convert to JPEG for compatibility
        try {
          const processedBase64 = await processImage(file)
          attachment.content = processedBase64
          // Ensure extension is jpg for consistency in naming (visual only)
          if (
            !attachment.name.toLowerCase().endsWith('.jpg') &&
            !attachment.name.toLowerCase().endsWith('.jpeg')
          ) {
            attachment.name =
              attachment.name.split('.').slice(0, -1).join('.') + '.jpg'
          }
          attachments = [...attachments, attachment]

          // Clean input message to remove any attachment references
          const cleanedInput = cleanInputMessage(inputMessage)
          if (cleanedInput !== inputMessage) {
            onInputChange(cleanedInput)
          }
          onAttachmentsChange?.(attachments)
        } catch (err) {
          console.error('❌ Failed to process image:', err)
        }

        target.value = ''
        return
      }

      // Add attachment for text and PDF
      attachments = [...attachments, attachment]
      // Clean input message to remove any attachment references
      const cleanedInput = cleanInputMessage(inputMessage)
      if (cleanedInput !== inputMessage) {
        onInputChange(cleanedInput)
      }
      onAttachmentsChange?.(attachments)

      // Reset file input
      target.value = ''
    } catch (err) {
      console.error(`❌ Failed to process ${fileType} file:`, err)
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

  const getFileIcon = (type: string): string => {
    switch (type) {
      case 'text':
        return 'note-text'
      case 'pdf':
        return 'file-pdf-box'
      case 'image':
        return 'image'
      case 'audio':
        return 'microphone'
      default:
        return 'file'
    }
  }
  // Helper to process, resize, and convert images to JPEG
  const processImage = (file: File): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = (e) => {
        const img = new window.Image()
        img.onload = () => {
          // Max dimension
          const MAX_SIZE = 1536
          let width = img.width
          let height = img.height

          if (width > MAX_SIZE || height > MAX_SIZE) {
            if (width > height) {
              height = Math.round((height * MAX_SIZE) / width)
              width = MAX_SIZE
            } else {
              width = Math.round((width * MAX_SIZE) / height)
              height = MAX_SIZE
            }
          }

          const canvas = document.createElement('canvas')
          canvas.width = width
          canvas.height = height
          const ctx = canvas.getContext('2d')
          if (!ctx) {
            reject(new Error('Failed to get canvas context'))
            return
          }

          // Draw with white background (for transparency handling)
          ctx.fillStyle = '#FFFFFF'
          ctx.fillRect(0, 0, width, height)
          ctx.drawImage(img, 0, 0, width, height)

          // Convert to JPEG with moderate quality
          const dataUrl = canvas.toDataURL('image/jpeg', 0.85)
          resolve(dataUrl)
        }
        img.onerror = () => reject(new Error('Failed to load image'))
        img.src = e.target?.result as string
      }
      reader.onerror = () => reject(new Error('Failed to read file'))
      reader.readAsDataURL(file)
    })
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
      rows="2"
    ></textarea>

    {#if attachments.length > 0}
      <div class="attachments-preview">
        {#each attachments as attachment, index (attachment.name + index)}
          <div class="attachment-chip">
            <MaterialIcon
              name={getFileIcon(attachment.type)}
              width="16"
              height="16"
            />
            <span class="attachment-name">{attachment.name}</span>
            <button
              type="button"
              class="remove-attachment"
              onclick={() => removeAttachment(index)}
              title="Remove attachment"
            >
              <MaterialIcon name="close" width="14" height="14" />
            </button>
          </div>
        {/each}
      </div>
    {/if}

    <div class="utility-bar">
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

      <div class="send-button-wrapper">
        <Button
          variant="primary"
          onclick={() => {
            // Clean input message before sending (remove any attachment references)
            const cleanedInput = cleanInputMessage(inputMessage)
            if (cleanedInput !== inputMessage) {
              onInputChange(cleanedInput)
            }
            onSend()
            clearAttachments()
          }}
          disabled={loading ||
            (!inputMessage.trim() && attachments.length === 0)}
          class="send-button"
        >
          <MaterialIcon name="send" width="20" height="20" />
        </Button>
      </div>
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
    flex-direction: column;
    gap: 0.5rem;
    max-width: 100%;
    margin: 0 auto;
    background-color: var(--bg-primary, #fff);
    border: 2px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    padding: 1rem;
    transition: all 0.2s ease;
    min-height: 60px;
  }

  .input-wrapper:focus-within {
    border-color: var(--accent-color, #2196f3);
    box-shadow: 0 0 0 3px rgba(33, 150, 243, 0.1);
  }

  .chat-input {
    flex: 1;
    padding: 0.75rem;
    border: none;
    background: transparent;
    font-family: inherit;
    font-size: 1rem;
    resize: none;
    min-height: 2.5rem;
    max-height: 150px;
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

  .attachments-preview {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    padding: 0 0.75rem 0.5rem;
  }

  .attachment-chip {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.75rem;
    background-color: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    font-size: 0.875rem;
    color: var(--text-primary, #100f0f);
  }

  .attachment-name {
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove-attachment {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: none;
    background: transparent;
    border-radius: 8px;
    cursor: pointer;
    color: var(--text-secondary, #666);
    padding: 0;
    transition: all 0.2s ease;
  }

  .remove-attachment:hover {
    background-color: var(--bg-tertiary, #e0e0e0);
    color: var(--text-primary, #100f0f);
  }

  .utility-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 0.5rem;
    gap: 1rem;
    border-top: 1px solid var(--border-color, #e0e0e0);
    margin-top: 0.5rem;
  }

  .file-buttons {
    display: flex;
    flex-direction: row;
    gap: 0.5rem;
    align-items: center;
    justify-content: flex-start;
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

  .send-button-wrapper {
    display: flex;
    align-items: center;
  }

  @media screen and (max-width: 768px) {
    .chat-input-container {
      padding: 1rem;
    }

    .input-wrapper {
      padding: 0.75rem;
      min-height: 50px;
    }

    .file-buttons {
      gap: 0.5rem;
    }

    .file-button {
      width: 32px;
      height: 32px;
    }

    .chat-input {
      font-size: 0.9rem;
      padding: 0.5rem;
      max-height: 120px;
    }

    .attachment-name {
      max-width: 100px;
    }
  }
</style>
