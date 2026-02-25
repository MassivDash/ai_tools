<script lang="ts">
  interface Props {
    onSubmit: (_answer: string) => void
    disabled?: boolean
  }

  let { onSubmit, disabled = false }: Props = $props()
  let answer = $state('')

  function handleSubmit() {
    if (!answer.trim() || disabled) return
    onSubmit(answer)
    answer = ''
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleSubmit()
    }
  }
</script>

<div class="answer-section">
  <input
    type="text"
    bind:value={answer}
    placeholder="Type your answer..."
    {disabled}
    onkeydown={handleKeydown}
    class="answer-input"
  />
  <button onclick={handleSubmit} {disabled} class="submit-btn"> Submit </button>
</div>

<style>
  .answer-section {
    display: flex;
    gap: 1rem;
    width: 100%;
    max-width: 500px;
    margin-top: 2rem;
  }

  .answer-input {
    flex-grow: 1;
    padding: 1rem 1.5rem;
    border-radius: 50px;
    border: 2px solid var(--border-color);
    background: rgba(0, 0, 0, 0.2);
    color: var(--text-primary);
    font-size: 1.1rem;
    transition: all 0.2s ease;
  }

  .answer-input:focus {
    outline: none;
    border-color: var(--primary-color);
    background: rgba(0, 0, 0, 0.4);
  }

  .submit-btn {
    padding: 0 2rem;
    border-radius: 50px;
    border: none;
    background: var(--primary-color);
    color: white;
    font-weight: 600;
    font-size: 1rem;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .submit-btn:hover:not(:disabled) {
    background: var(--primary-hover);
    transform: translateY(-2px);
  }

  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
