<script lang="ts">
  import type { ChatMessage } from '../types'
  import MessageItem from './MessageItem.svelte'

  export let messages: ChatMessage[] = []
  export let loading: boolean = false
  export let chatContainer: HTMLDivElement
</script>

<div class="chat-messages" bind:this={chatContainer}>
  {#if messages.length === 0}
    <div class="empty-chat">
      <p>👋 Start a conversation with the AI agent</p>
      <p class="hint">
        Ask questions and the agent will use its tools to help you
      </p>
    </div>
  {:else}
    {#each messages as message (message.id)}
      <MessageItem {message} />
    {/each}
    {#if loading}
      <div class="message assistant">
        <div class="message-role">Assistant</div>
        <div class="message-content loading">
          <span class="typing-indicator">
            <span></span>
            <span></span>
            <span></span>
          </span>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    scroll-behavior: smooth;
  }

  .chat-messages::-webkit-scrollbar {
    width: 8px;
  }

  .chat-messages::-webkit-scrollbar-track {
    background: var(--bg-secondary, #f5f5f5);
    border-radius: 4px;
  }

  .chat-messages::-webkit-scrollbar-thumb {
    background: var(--border-color, #ddd);
    border-radius: 4px;
  }

  .chat-messages::-webkit-scrollbar-thumb:hover {
    background: var(--text-secondary, #999);
  }

  .empty-chat {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary, #666);
    text-align: center;
    padding: 2rem;
  }

  .empty-chat .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary, #999);
    margin-top: 0.5rem;
  }

  .message {
    display: flex;
    flex-direction: column;
    max-width: 80%;
    padding: 0 1rem;
  }

  .message.assistant {
    align-self: flex-start;
  }

  .message-role {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary, #666);
    margin-bottom: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .message-content.loading {
    background-color: transparent;
    border: none;
    padding: 0.5rem;
  }

  .typing-indicator {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }

  .typing-indicator span {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--text-secondary, #999);
    animation: typing 1.4s infinite;
  }

  .typing-indicator span:nth-child(2) {
    animation-delay: 0.2s;
  }

  .typing-indicator span:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes typing {
    0%,
    60%,
    100% {
      transform: translateY(0);
      opacity: 0.7;
    }
    30% {
      transform: translateY(-10px);
      opacity: 1;
    }
  }

  @media screen and (max-width: 768px) {
    .chat-messages {
      padding: 0;
    }
  }
</style>
