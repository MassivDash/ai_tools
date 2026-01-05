<script lang="ts">
  import Timer from '../Timer.svelte'
  import QuestionDisplay from '../QuestionDisplay.svelte'
  import AnswerInput from '../AnswerInput.svelte'
  import MaterialIcon from '../../../ui/MaterialIcon.svelte'
  import type { Question } from '../../../../hooks/useOneOfFifteenState.svelte'

  interface Props {
    isActivePlayer: boolean
    timerStart: number | null | undefined
    duration: number
    currentQuestion: Question | null | undefined
    questionNumber?: number
    activePlayerName: string
    onTimeout: () => void
    onSubmitAnswer: (_answer: string) => void
  }

  let {
    isActivePlayer,
    timerStart,
    duration,
    currentQuestion,
    questionNumber,
    activePlayerName,
    onTimeout,
    onSubmitAnswer
  }: Props = $props()
</script>

{#if isActivePlayer}
  <div class="your-turn-panel">
    <h3>IT'S YOUR TURN!</h3>

    <Timer startTime={timerStart} {duration} {onTimeout} />

    {#if currentQuestion}
      <QuestionDisplay questionText={currentQuestion.text} {questionNumber} />
      <AnswerInput onSubmit={onSubmitAnswer} />
    {:else}
      <div class="generating-message">
        <MaterialIcon name="cog-outline" class="spin" />
        <p>Generating question...</p>
      </div>
    {/if}
  </div>
{:else}
  <div class="waiting-turn">
    <h3>Waiting for other players...</h3>
    <p>
      Active Player: {activePlayerName}
    </p>
    {#if currentQuestion}
      <QuestionDisplay questionText={currentQuestion.text} />
    {/if}
  </div>
{/if}

<style>
  .your-turn-panel {
    background: var(--bg-secondary);
    padding: 2rem;
    border-radius: 12px;
    border: 2px solid var(--primary);
    width: 100%;
    max-width: 800px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
    animation: slideUp 0.3s ease-out;
  }

  .waiting-turn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    width: 100%;
    max-width: 800px;
  }

  .generating-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem;
    opacity: 0.7;
  }

  @keyframes slideUp {
    from {
      transform: translateY(20px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }
</style>
