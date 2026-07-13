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
</style>
