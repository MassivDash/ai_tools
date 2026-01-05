<script lang="ts">
  import Round2Pointing from './Round2Pointing.svelte'
  import Round3Buzzer from './Round3Buzzer.svelte'
  import Round3Decision from './Round3Decision.svelte'
  import QuestionAnswering from './QuestionAnswering.svelte'
  import MaterialIcon from '../../../ui/MaterialIcon.svelte' // Helper for decider message
  import type { GameStateSnapshot } from '../../../../hooks/useOneOfFifteenState.svelte'

  interface Props {
    gameState: GameStateSnapshot
    sessionId: string
    isRound2: boolean
    isRound3: boolean
    isPointingPhase: boolean
    isMyTurnToPoint: boolean
    isBuzzerPhase: boolean
    isDecisionPhase: boolean
    isActivePlayer: boolean
    pointerName: string
    deciderName: string
    activePlayerName: string
    questionNumber?: number
    // Actions
    pointToPlayer: (id: string) => void
    buzzIn: () => void
    makeDecision: (choice: 'self' | 'point', targetId?: string) => void
    onTimeout: () => void
    onSubmitAnswer: (answer: string) => void
  }

  let {
    gameState,
    sessionId,
    isRound2,
    isRound3,
    isPointingPhase,
    isMyTurnToPoint,
    isBuzzerPhase,
    isDecisionPhase,
    isActivePlayer,
    pointerName,
    deciderName,
    activePlayerName,
    questionNumber,
    pointToPlayer,
    buzzIn,
    makeDecision,
    onTimeout,
    onSubmitAnswer
  }: Props = $props()
</script>

<div class="game-view">
  {#if isBuzzerPhase}
    <Round3Buzzer onBuzzIn={buzzIn} />
  {:else if isDecisionPhase}
    <Round3Decision
      players={gameState.contestants}
      myId={sessionId}
      onMakeDecision={makeDecision}
    />
  {:else if isRound3 && gameState.decision_pending && !isActivePlayer}
    <!-- Spectating Decision -->
    <div class="spectator-view">
      <div class="generating-message">
        <MaterialIcon name="source-branch" width="48" height="48" />
        <h3>Decision Time</h3>
        <p>{deciderName} is making a decision...</p>
      </div>
    </div>
  {:else if isPointingPhase}
    <Round2Pointing
      {isMyTurnToPoint}
      players={gameState.contestants}
      myId={sessionId}
      {pointerName}
      onPointToPlayer={pointToPlayer}
    />
  {:else}
    <QuestionAnswering
      {isActivePlayer}
      timerStart={gameState.timer_start}
      duration={60}
      currentQuestion={gameState.current_question}
      {questionNumber}
      {activePlayerName}
      {onTimeout}
      {onSubmitAnswer}
    />
  {/if}
</div>

<style>
  .game-view {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
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
