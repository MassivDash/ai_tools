<script lang="ts">
  interface Props {
    questionText: string
    questionNumber?: number
  }

  let { questionText, questionNumber }: Props = $props()
</script>

<div class="question-card">
  {#if questionNumber}
    <div class="question-header">Question {questionNumber}</div>
  {/if}
  <div class="question-text">
    {questionText}
  </div>
</div>

<style>
  .question-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 2rem;
    margin: 1rem 0;
    text-align: center;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
    max-width: 600px;
    width: 100%;
  }

  .question-header {
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 2px;
    color: var(--text-secondary);
    margin-bottom: 1rem;
  }

  .question-text {
    font-size: 1.5rem;
    font-weight: 600;
    line-height: 1.4;
    color: var(--text-primary);
  }
</style>
