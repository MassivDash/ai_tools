<script lang="ts">
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import { useOneOfFifteenState } from '../../../hooks/useOneOfFifteenState.svelte'
  import type { GameStateSnapshot } from '../../../hooks/useOneOfFifteenState.svelte'

  import Timer from './Timer.svelte'
  import QuestionDisplay from './QuestionDisplay.svelte'
  import AnswerInput from './AnswerInput.svelte'
  import PlayerGrid from './PlayerGrid.svelte'

  interface Props {
    gameState: GameStateSnapshot
    contestantName: string
    sessionId: string
    onToggleReady: () => void
    onSubmitAnswer: (_answer: string) => void
  }

  let {
    gameState,
    contestantName,
    sessionId,
    onToggleReady,
    onSubmitAnswer
  }: Props = $props()

  // Destructure hook for additional actions
  const { pointToPlayer, buzzIn, makeDecision } = useOneOfFifteenState()

  let myContestant = $derived(
    gameState.contestants.find(
      (c) => c.id === sessionId || c.session_id === sessionId
    )
  )
  let isReady = $derived(myContestant?.ready || false)
  let isActivePlayer = $derived(gameState.active_player_id === sessionId)
  let isEliminated = $derived(myContestant?.eliminated || false)

  // Round Logic
  let isRound2 = $derived(gameState.round === 'round2')
  let isRound3 = $derived(gameState.round === 'round3')

  // Round 2 Pointing
  let isPointingPhase = $derived(isRound2 && !gameState.current_question)
  let isMyTurnToPoint = $derived(isActivePlayer && isPointingPhase)

  // Round 3 Buzzer & Decision
  let isBuzzerPhase = $derived(isRound3 && !gameState.active_player_id)
  let isDecisionPhase = $derived(
    isRound3 && isActivePlayer && gameState.decision_pending
  )

  // Local state for Decision flow (choosing "Point" -> showing Grid)
  let showDecisionGrid = $state(false)

  function handleTimeout() {
    if (isActivePlayer && (gameState.round === 'round1' || isRound3)) {
      // Timeout in Round 3 also bad, maybe? Or logic handled by backend?
      // Backend doesn't have explicit timeout message yet except implicit time check.
      // Sending timeout answer is safe fallback.
      onSubmitAnswer('!!!TIMEOUT!!!')
    }
  }

  let getStatusMessage = () => {
    if (gameState.round === 'lobby') return 'Waiting for game to start...'
    if (isEliminated) return 'ELIMINATED'

    // Round 2
    if (isPointingPhase) {
      if (isMyTurnToPoint) return 'Pick a Player!'
      let pointerName =
        gameState.contestants.find((c) => c.id === gameState.active_player_id)
          ?.name || 'Unknown'
      return `Waiting for ${pointerName} to point...`
    }

    // Round 3
    if (isRound3) {
      if (isBuzzerPhase) return 'BUZZ TO ANSWER!'
      if (isDecisionPhase) return 'Make a Decision!'
      if (isActivePlayer) return 'YOUR TURN!'
      if (gameState.decision_pending) {
        let deciderName =
          gameState.contestants.find((c) => c.id === gameState.active_player_id)
            ?.name || 'Unknown'
        return `${deciderName} is deciding...`
      }
    }

    if (isActivePlayer) return 'YOUR TURN!'

    let activeName =
      gameState.contestants.find((c) => c.id === gameState.active_player_id)
        ?.name || 'Unknown'
    return `${activeName} is answering...`
  }
</script>

<div class="contestant-dashboard">
  <h2>Welcome, {contestantName || myContestant?.name || 'Player'}!</h2>

  <div class="header-status">
    <div
      class="status-badge {isActivePlayer ? 'active' : ''} {isEliminated
        ? 'eliminated'
        : ''}"
    >
      {getStatusMessage()}
    </div>
  </div>

  {#if gameState.round === 'lobby'}
    <div class="waiting-screen">
      {#if isReady}
        <div class="spinner-box">
          <MaterialIcon
            name="clock-outline"
            width="48"
            height="48"
            class="spin"
          />
        </div>
        <h3>Waiting for Presenter to start...</h3>
        <p class="status-sub">You are ready!</p>
        <button class="btn-link" onclick={onToggleReady}>Not Ready?</button>
      {:else}
        <div class="ready-prompt">
          <h3>Are you ready to play?</h3>
          <p>Click the button below when you are ready.</p>
          <button class="btn-ready" onclick={onToggleReady}>
            I'M READY!
          </button>
        </div>
      {/if}
    </div>
  {:else if isEliminated}
    <div class="eliminated-view">
      <MaterialIcon name="close-circle" width="64" height="64" />
      <h2>ELIMINATED</h2>
      <p>You have been eliminated from the game.</p>
      {#if isRound3}
        <p>Current Round: The Buzzer!</p>
      {/if}
      <div class="final-score">Final Score: {myContestant?.score}</div>
    </div>
  {:else if gameState.round !== 'finished'}
    <div class="game-active-screen">
      {#if isBuzzerPhase}
        <!-- BUZZER UI -->
        <div class="buzzer-container">
          <button class="buzzer-btn" onclick={buzzIn}> BUZZ! </button>
          <p>Be the first to buzz in!</p>
        </div>
      {:else if isDecisionPhase}
        <!-- DECISION UI -->
        <div class="decision-container">
          <h3>Correct! What do you want to do?</h3>
          {#if !showDecisionGrid}
            <div class="decision-buttons">
              <button
                class="btn-decision self"
                onclick={() => makeDecision('self')}
              >
                Double Down (Self)
              </button>
              <button
                class="btn-decision point"
                onclick={() => (showDecisionGrid = true)}
              >
                Point to Player
              </button>
            </div>
          {:else}
            <!-- Choosing a player to point to -->
            <div class="pointing-container">
              <h4>Select a player to answer:</h4>
              <PlayerGrid
                players={gameState.contestants}
                excludeId={sessionId}
                onSelect={(id) => makeDecision('point', id)}
              />
              <button
                class="btn-link"
                onclick={() => (showDecisionGrid = false)}>Back</button
              >
            </div>
          {/if}
        </div>
      {:else if isPointingPhase && isMyTurnToPoint}
        <!-- Round 2 Pointing UI -->
        <div class="pointing-container">
          <h3>It's your turn to choose the next player!</h3>
          <PlayerGrid
            players={gameState.contestants}
            excludeId={sessionId}
            onSelect={(id) => {
              pointToPlayer(id)
            }}
          />
        </div>
      {:else if isPointingPhase}
        <!-- Spectating Pointing -->
        <div class="spectator-view">
          <div class="generating-message">
            <MaterialIcon
              name="account-search-outline"
              width="48"
              height="48"
            />
            <h3>Pointing Phase</h3>
            <p>Waiting for player selection...</p>
          </div>
        </div>
      {:else if isActivePlayer}
        <div class="your-turn-panel">
          <h3>IT'S YOUR TURN!</h3>

          <Timer
            startTime={gameState.timer_start}
            duration={60}
            onTimeout={handleTimeout}
          />

          {#if gameState.current_question}
            <QuestionDisplay
              questionText={gameState.current_question.text}
              questionNumber={myContestant?.round1_questions !== undefined
                ? myContestant.round1_questions + 1
                : undefined}
            />
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
            Active Player: {gameState.contestants.find(
              (c) => c.id === gameState.active_player_id
            )?.name || gameState.active_player_id}
          </p>
          {#if gameState.current_question}
            <QuestionDisplay questionText={gameState.current_question.text} />
          {/if}
        </div>
      {/if}

      <div class="stats-footer">
        <div class="stat-box">
          <span class="label">Score</span>
          <span class="value">{myContestant?.score || 0}</span>
        </div>
        <div class="stat-box">
          <span class="label">Lives</span>
          <span class="value">{'❤️'.repeat(myContestant?.lives || 0)}</span>
        </div>
        {#if gameState.round === 'round1'}
          <div class="stat-box">
            <span class="label">Strikes</span>
            <span class="value">{myContestant?.round1_misses || 0}/2</span>
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <!-- Finished or other state -->
    <div class="waiting-screen">
      <h3>Game Ended.</h3>
    </div>
  {/if}

  {#if gameState.has_presenter}
    <div
      class="presenter-status {gameState.presenter_online
        ? 'online'
        : 'offline'}"
    >
      {#if gameState.presenter_online}
        <MaterialIcon name="check-circle" width="16" height="16" /> Presenter Online
      {:else}
        <MaterialIcon name="alert-circle" width="16" height="16" /> Presenter Offline
      {/if}
    </div>
  {:else}
    <div class="presenter-status offline">
      <MaterialIcon name="alert-circle" width="16" height="16" /> Presenter Offline
    </div>
  {/if}
</div>

<style>
  .contestant-dashboard {
    text-align: center;
    padding: 2rem;
    height: 100vh;
    display: flex;
    flex-direction: column;
    color: var(--text-primary);
  }

  .header-status {
    margin-bottom: 1rem;
  }
  .status-badge {
    display: inline-block;
    padding: 0.5rem 1rem;
    border-radius: 999px;
    background: var(--surface-2);
    font-weight: bold;
    font-size: 0.9rem;
  }
  .status-badge.active {
    background: var(--primary);
    color: var(--text-primary-inverse);
  }
  .status-badge.eliminated {
    background: var(--error);
    color: var(--text-primary-inverse);
  }

  /* Lobby Styles */
  .waiting-screen {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  .eliminated-view {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--error);
  }
  .eliminated-view h2 {
    margin: 1rem 0 0.5rem;
    font-size: 2rem;
  }
  .final-score {
    font-size: 1.5rem;
    font-weight: bold;
    color: var(--text-primary);
  }

  .game-active-screen {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center; /* Center everything */
    justify-content: center;
    padding-bottom: 80px; /* Space for footer */
    width: 100%;
  }

  .your-turn-panel,
  .pointing-container,
  .decision-container {
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

  .pointing-container {
    border-color: var(--accent);
  }

  .decision-container {
    border-color: var(--warning);
  }

  .buzzer-container {
    padding: 3rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .buzzer-btn {
    width: 200px;
    height: 200px;
    border-radius: 50%;
    background: var(--error);
    color: white;
    font-size: 2.5rem;
    font-weight: bold;
    border: 8px solid #c00;
    cursor: pointer;
    box-shadow: 0 4px 15px rgba(0, 0, 0, 0.3);
    transition:
      transform 0.1s,
      box-shadow 0.1s;
  }
  .buzzer-btn:active {
    transform: scale(0.95);
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.3);
  }

  .decision-buttons {
    display: flex;
    gap: 2rem;
  }
  .btn-decision {
    padding: 1.5rem 2rem;
    font-size: 1.2rem;
    border-radius: 8px;
    border: none;
    cursor: pointer;
    font-weight: bold;
    color: white;
  }
  .btn-decision.self {
    background: var(--primary);
  }
  .btn-decision.point {
    background: var(--accent);
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

  .stats-footer {
    position: fixed;
    bottom: 0;
    left: 0;
    width: 100%;
    background: var(--bg-primary);
    border-top: 1px solid var(--border-color);
    display: flex;
    justify-content: space-around;
    padding: 1rem;
    box-shadow: 0 -2px 10px rgba(0, 0, 0, 0.1);
  }
  .stat-box {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .stat-box .label {
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 1px;
  }
  .stat-box .value {
    font-weight: bold;
    font-size: 1.25rem;
    margin-top: 0.25rem;
    color: var(--text-primary);
  }

  /* Utilities */
  .spinner-box {
    margin-bottom: 1rem;
    color: var(--text-secondary);
  }
  .ready-prompt {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    align-items: center;
  }
  .status-sub {
    color: var(--success);
    font-weight: bold;
  }
  .btn-link {
    background: none;
    border: none;
    text-decoration: underline;
    color: var(--text-secondary);
    cursor: pointer;
    margin-top: 1rem;
  }
  .btn-ready {
    background: var(--success);
    color: var(--text-primary-inverse, #fff);
    border: none;
    padding: 1rem 2rem;
    font-size: 1.2rem;
    font-weight: bold;
    border-radius: 8px;
    cursor: pointer;
    transition: transform 0.1s;
  }
  .btn-ready:active {
    transform: scale(0.95);
  }

  .presenter-status {
    position: absolute;
    top: 1rem;
    right: 1rem;
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 16px;
  }
  .presenter-status.online {
    color: var(--success);
  }
  .presenter-status.offline {
    color: var(--error);
  }
</style>
