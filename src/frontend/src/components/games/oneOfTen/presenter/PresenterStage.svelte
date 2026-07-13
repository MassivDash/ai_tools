<script lang="ts">
  import MaterialIcon from '../../../ui/MaterialIcon.svelte'
  import RobotPresenter from '../../robot/RobotPresenter.svelte'
  import type { Question } from '../../../../hooks/useOneOfTenState.svelte'

  interface Props {
    round: string
    currentQuestion: Question | null
    robotEmotion: string
    robotTalking: boolean
    timeLeft: number
    isIntroPlaying: boolean
  }

  let {
    round,
    currentQuestion,
    robotEmotion,
    robotTalking,
    timeLeft,
    isIntroPlaying
  }: Props = $props()
</script>

<div class="robot-panel">
  {#if round !== 'lobby' || isIntroPlaying}
    <RobotPresenter emotion={robotEmotion} talking={robotTalking} />

    <div class="game-status-bar">
      {#if round === 'round1'}
        <div class="timer-display" class:urgent={timeLeft < 10}>
          ⏰ {timeLeft}s
        </div>
      {/if}
      <div class="round-badge">
        {round.toUpperCase()}
      </div>
    </div>

    {#if currentQuestion}
      <div class="prompter-card">
        <h4>Current Question:</h4>
        <div class="question-text-lg">{currentQuestion.text}</div>
      </div>
    {/if}
  {:else}
    <div class="waiting-placeholder">
      <MaterialIcon name="robot-off" width="96" height="96" />
      <h2>Waiting for Game to Start...</h2>
      <p>The Robot Presenter will appear here.</p>
    </div>
  {/if}
</div>

<style>
  .robot-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    background: var(--bg-secondary);
    padding: 2rem;
    border-radius: 12px;
    height: 100%;
    justify-content: center;
    position: relative;
    border: 1px solid var(--border-color, rgba(255, 255, 255, 0.1));
  }

  .game-status-bar {
    position: absolute;
    top: 1rem;
    right: 1rem;
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  .timer-display {
    font-size: 2rem;
    font-weight: bold;
    background: var(--bg-primary);
    color: var(--text-primary);
    padding: 0.5rem 1rem;
    border-radius: 8px;
    border: 1px solid var(--border-color);
  }
  .timer-display.urgent {
    background: var(--error);
    color: var(--text-primary-inverse, #fff);
    animation: pulse 1s infinite;
  }

  @keyframes pulse {
    0% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
    100% {
      opacity: 1;
    }
  }

  .round-badge {
    background: var(--primary);
    color: var(--text-primary-inverse, #fff);
    padding: 0.5rem 1rem;
    border-radius: 8px;
    font-weight: bold;
  }

  .prompter-card {
    background: var(--bg-highlight, #fff8e1);
    border: 2px solid var(--border-highlight, #ffecb3);
    padding: 2rem;
    border-radius: 16px;
    width: 80%;
    text-align: center;
    margin-top: 1rem;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    color: var(--text-primary-inverse, #333);
  }

  .question-text-lg {
    font-size: 2rem;
    font-weight: 800;
    margin: 1rem 0;
    color: inherit;
  }

  .waiting-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    text-align: center;
    gap: 0.5rem;
    min-height: 200px;
  }
</style>
