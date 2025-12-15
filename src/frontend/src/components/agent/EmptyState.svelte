<div class="empty-state">
  <p>🤖 AI Agent is ready</p>
  <p class="hint">
    Click "Start Server" to launch the llama.cpp server and start chatting
    with the AI agent
  </p>
  <p class="hint-small">Server will be available at localhost:8080</p>
</div>

<style>
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary, #666);
    text-align: center;
    min-height: 80vh;
    transition: color 0.3s ease;
  }

  .empty-state p {
    margin: 0.5rem 0;
  }

  .empty-state .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary, #999);
    transition: color 0.3s ease;
  }

  .empty-state .hint-small {
    font-size: 0.8rem;
    color: var(--text-tertiary, #aaa);
    transition: color 0.3s ease;
  }
</style>

