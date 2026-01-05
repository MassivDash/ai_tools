<script lang="ts">
  import Button from '../../ui/Button.svelte'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import type { GameStateSnapshot } from '../../../hooks/useOneOfFifteenState.svelte'
  import RobotPresenter from '../robot/RobotPresenter.svelte'
  import { useTextToSpeech } from '../../../hooks/useTextToSpeech.svelte'

  interface Props {
    gameState: GameStateSnapshot
    onStartGame: () => void
    onResetGame: () => void
  }

  import { onDestroy } from 'svelte'

  let { gameState, onStartGame, onResetGame }: Props = $props()

  const tts = useTextToSpeech() // Default config
  let lastSpokenQuestion = $state('')
  let robotEmotion = $state('normal')
  let robotTalking = $state(false)
  let timeLeft = $state(60)
  let timerInterval: any

  $effect(() => {
    if (gameState.timer_start && gameState.round === 'round1') {
      clearInterval(timerInterval)
      timerInterval = setInterval(() => {
        const now = Date.now() / 1000
        const start = gameState.timer_start || 0
        const elapsed = now - start
        timeLeft = Math.max(0, 60 - Math.floor(elapsed))
      }, 1000)
    } else {
      clearInterval(timerInterval)
      timeLeft = 60
    }
    return () => clearInterval(timerInterval)
  })

  $effect(() => {
    // ... exisiting speech effect logic
    if (
      gameState.current_question &&
      gameState.current_question.text !== lastSpokenQuestion
    ) {
      lastSpokenQuestion = gameState.current_question.text
      robotEmotion = 'happy'
      robotTalking = true
      tts.speak(gameState.current_question.text)
      setTimeout(() => {
        robotTalking = false
        robotEmotion = 'normal'
      }, 3000)
    }
  })
</script>

<div class="presenter-dashboard">
  <!-- Robot Presenter Panel - Top Full Width -->
  <div class="robot-panel">
    {#if gameState.round !== 'lobby'}
      <RobotPresenter emotion={robotEmotion} talking={robotTalking} />

      <div class="game-status-bar">
        {#if gameState.round === 'round1'}
          <div class="timer-display" class:urgent={timeLeft < 10}>
            ⏰ {timeLeft}s
          </div>
        {/if}
        <div class="round-badge">
          {gameState.round.toUpperCase()}
        </div>
      </div>

      {#if gameState.current_question}
        <div class="prompter-card">
          <h4>Current Question:</h4>
          <div class="question-text-lg">{gameState.current_question.text}</div>
          <div class="answer-key">
            <strong>Answer:</strong>
            {gameState.current_question.correct_answer}
          </div>
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

  <div class="dashboard-grid">
    <div class="controls-panel">
      <h3>Game Controls</h3>
      <div class="control-buttons">
        {#if gameState.round === 'lobby'}
          <Button
            variant="success"
            onclick={onStartGame}
            disabled={gameState.contestants.length === 0}
          >
            <MaterialIcon name="play" width="24" height="24" />
            Start Game
          </Button>
          <p class="status-text">
            Status: Lobby ({gameState.contestants.length} players)
          </p>
        {:else if gameState.round !== 'finished'}
          <Button variant="danger" onclick={onResetGame}>
            <MaterialIcon name="refresh" width="24" height="24" />
            Reset Game
          </Button>
          <p class="status-text active">
            Status: {gameState.round} Active
          </p>
        {/if}
      </div>
    </div>

    <div class="contestants-list">
      <h3>
        Contestants ({gameState.contestants.length})
      </h3>
      <ul>
        {#each gameState.contestants as contestant}
          <li
            class:online={contestant.online}
            class:offline={!contestant.online}
            class:eliminated={contestant.eliminated}
          >
            <div class="c-info">
              <span class="c-name">
                {contestant.name}
                {#if contestant.age}
                  <span class="c-age">({contestant.age})</span>
                {/if}
              </span>
              {#if contestant.eliminated}
                <span class="badge u-eliminated">ELIMINATED</span>
              {/if}
            </div>

            <div class="c-stats">
              {#if gameState.round === 'round1'}
                <span class="stat-pill" title="Misses"
                  >❌ {contestant.round1_misses}/2</span
                >
              {/if}
              <span class="stat-pill">❤️ {contestant.lives}</span>
              <span class="stat-pill">⭐ {contestant.score}</span>
            </div>
          </li>
        {/each}
      </ul>
    </div>
  </div>
</div>

<style>
  .presenter-dashboard {
    text-align: left;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    height: 100%;
    color: var(--text-primary);
  }

  .robot-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    background: var(--bg-secondary);
    padding: 2rem;
    border-radius: 12px;
    flex-grow: 1; /* Take available space */
    min-height: 400px;
    /* Removed max-height and overflow hidden to allow bubble visibility */
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
    color: var(
      --text-primary-inverse,
      #333
    ); /* Force dark text on light card if highlight is light */
  }

  .question-text-lg {
    font-size: 2rem;
    font-weight: 800;
    margin: 1rem 0;
    color: inherit;
  }

  .dashboard-grid {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 1.5rem;
  }

  .controls-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    padding: 1rem;
    border-radius: 8px;
  }

  .contestants-list {
    text-align: left;
    background: var(--bg-secondary);
    padding: 1rem;
    border-radius: 8px;
    height: 100%;
    border: 1px solid var(--border-color);
  }

  .contestants-list ul {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .contestants-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--text-primary);
  }
  .contestants-list li.eliminated {
    opacity: 0.5;
    background: var(--bg-secondary);
    text-decoration: line-through;
  }
  .contestants-list li.offline {
    opacity: 0.6;
    background: var(--bg-secondary);
  }

  .c-name {
    font-weight: 600;
  }

  .c-age {
    font-weight: 400;
    color: var(--text-secondary);
    margin-left: 0.5rem;
    font-size: 0.9em;
  }

  .badge.u-eliminated {
    background: var(--error);
    color: var(--text-primary-inverse, #fff);
  }

  .c-stats {
    display: flex;
    gap: 0.5rem;
  }
  .stat-pill {
    background: var(--bg-secondary);
    padding: 0.25rem 0.5rem;
    border-radius: 12px;
    font-size: 0.85rem;
    font-weight: bold;
    border: 1px solid var(--border-color);
  }

  .status-text {
    font-size: 0.9rem;
    color: var(--text-secondary);
    margin: 0;
  }
  .status-text.active {
    color: var(--success);
    font-weight: bold;
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
