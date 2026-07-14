<script lang="ts">
  import type { GameStateSnapshot } from '../../../hooks/useOneOfTenState.svelte'
  import {
    useContestantLogic,
    PHASE
  } from '../../../hooks/useContestantLogic.svelte'

  // Sub-components
  import ContestantLayout from './contestant/ContestantLayout.svelte'
  import LobbyView from './contestant/LobbyView.svelte'
  import EliminatedView from './contestant/EliminatedView.svelte'
  import QuestionPhase from './contestant/QuestionPhase.svelte'
  import Round2Pointing from './contestant/Round2Pointing.svelte'
  import Round3Buzzer from './contestant/Round3Buzzer.svelte'
  import Round3Decision from './contestant/Round3Decision.svelte'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'

  interface Props {
    gameState: GameStateSnapshot
    contestantName: string
    sessionId: string
    onToggleReady: () => void
    onSubmitAnswer: (_answer: string) => void
    // Actions passed from parent
    pointToPlayer: (_id: string) => void
    buzzIn: () => void
    makeDecision: (_choice: 'self' | 'point', _targetId?: string) => void
  }

  let {
    gameState,
    contestantName,
    sessionId,
    onToggleReady,
    onSubmitAnswer,
    pointToPlayer,
    buzzIn,
    makeDecision
  }: Props = $props()

  // Logic Hook

  const logic = useContestantLogic(() => gameState, sessionId)

  function handleTimeout() {
    if (logic.isActivePlayer) {
      onSubmitAnswer('!!!TIMEOUT!!!')
    }
  }
</script>

<ContestantLayout
  {contestantName}
  statusMessage={logic.statusMessage}
  isActivePlayer={logic.isActivePlayer}
  isEliminated={logic.isEliminated}
  hasPresenter={gameState.has_presenter}
  presenterOnline={gameState.presenter_online}
  score={logic.myContestant?.score || 0}
  lives={logic.myContestant?.lives || 0}
  isRound1={logic.isRound1}
  round1Misses={logic.myContestant?.round1_misses}
  hideFooter={gameState.round === 'lobby'}
>
  {#if logic.currentPhase === PHASE.LOBBY}
    <LobbyView isReady={logic.isReady} {onToggleReady} />
  {:else if logic.currentPhase === PHASE.ELIMINATED}
    <EliminatedView
      score={logic.myContestant?.score || 0}
      isRound3={logic.isRound3}
    />
  {:else if logic.currentPhase === PHASE.POINTING}
    <Round2Pointing
      isMyTurnToPoint={logic.isMyTurnToPoint}
      players={gameState.contestants}
      myId={sessionId}
      pointerName={logic.pointerName}
      onPointToPlayer={pointToPlayer}
    />
  {:else if logic.currentPhase === PHASE.BUZZER}
    <Round3Buzzer onBuzzIn={buzzIn} />
  {:else if logic.currentPhase === PHASE.DECISION}
    <Round3Decision
      players={gameState.contestants}
      myId={sessionId}
      onMakeDecision={makeDecision}
    />
  {:else if logic.currentPhase === PHASE.SPECTATING_DECISION}
    <div class="spectator-view">
      <div class="generating-message">
        <MaterialIcon name="source-branch" width="48" height="48" />
        <h3>Decision Time</h3>
        <p>{logic.deciderName} is making a decision...</p>
      </div>
    </div>
  {:else if logic.currentPhase === PHASE.WAITING_FOR_PRESENTER}
    <div class="spectator-view">
      <div class="generating-message">
        <MaterialIcon name="microphone" width="48" height="48" />
        <h3>Host Speaking</h3>
        <p>Listen to the presenter...</p>
      </div>
    </div>
  {:else if logic.currentPhase === PHASE.ANSWERING || logic.currentPhase === PHASE.WAITING}
    <QuestionPhase
      isActivePlayer={logic.isActivePlayer}
      timerStart={gameState.timer_start}
      duration={60}
      currentQuestion={gameState.current_question}
      questionNumber={logic.myContestant?.round1_questions !== undefined
        ? logic.myContestant.round1_questions + 1
        : undefined}
      activePlayerName={logic.activePlayerName}
      onTimeout={handleTimeout}
      {onSubmitAnswer}
    />
  {:else if logic.currentPhase === PHASE.FINISHED}
    <div class="finished-screen">
      <div class="trophy-container">🏆</div>
      <h2>Game Over</h2>
      {#if gameState.winner_id === sessionId}
        <div class="winner-message victory">
          <h3>Congratulations! 🎉</h3>
          <p>You are the winner of 1 z 10!</p>
          <div class="final-score">
            Your Score: <strong>{logic.myContestant?.score || 0}</strong> points
          </div>
        </div>
      {:else}
        <div class="winner-message">
          <h3>Winner Announcement</h3>
          <p class="winner-name">
            Winner: <strong
              >{gameState.contestants.find((c) => c.id === gameState.winner_id)
                ?.name || 'Unknown'}</strong
            >
          </p>
          <p class="winner-points">
            Score: {gameState.contestants.find(
              (c) => c.id === gameState.winner_id
            )?.score || 0} points
          </p>
        </div>
      {/if}
      <div class="stats-summary">
        <p>Thank you for playing!</p>
        <p>Your Final Score: {logic.myContestant?.score || 0} pts</p>
      </div>
    </div>
  {:else}
    <div class="waiting-screen">
      <h3>Game Ended.</h3>
    </div>
  {/if}
</ContestantLayout>

<style>
  .waiting-screen {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .generating-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem;
    opacity: 0.7;
  }

  .finished-screen {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1.5rem;
    padding: 2rem;
    text-align: center;
    color: var(--text-primary);
  }

  .trophy-container {
    font-size: 5rem;
    animation: bounce 2s infinite;
  }

  .winner-message {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-color, rgba(255, 255, 255, 0.1));
    border-radius: 16px;
    padding: 2rem;
    width: 100%;
    max-width: 400px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
  }

  .winner-message.victory {
    background: linear-gradient(
      135deg,
      rgba(255, 215, 0, 0.15),
      rgba(255, 165, 0, 0.1)
    );
    border: 1px solid #ffd700;
    box-shadow: 0 4px 25px rgba(255, 215, 0, 0.2);
  }

  .winner-message h3 {
    margin-top: 0;
    font-size: 1.6rem;
    color: var(--primary);
  }

  .winner-message.victory h3 {
    color: #ffd700;
    text-shadow: 0 0 10px rgba(255, 215, 0, 0.3);
  }

  .final-score {
    margin-top: 1rem;
    font-size: 1.2rem;
  }

  .winner-name {
    font-size: 1.3rem;
    margin: 0.5rem 0;
  }

  .stats-summary {
    margin-top: 1rem;
    opacity: 0.8;
    font-size: 0.95rem;
  }

  @keyframes bounce {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-15px);
    }
  }
</style>
