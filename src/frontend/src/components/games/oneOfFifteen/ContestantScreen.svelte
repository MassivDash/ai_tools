<script lang="ts">
  import type { GameStateSnapshot } from '../../../hooks/useOneOfFifteenState.svelte'

  // Sub-components
  import ContestantLayout from './contestant/ContestantLayout.svelte'
  import LobbyView from './contestant/LobbyView.svelte'
  import EliminatedView from './contestant/EliminatedView.svelte'
  import GameView from './contestant/GameView.svelte'

  interface Props {
    gameState: GameStateSnapshot
    contestantName: string
    sessionId: string
    onToggleReady: () => void
    onSubmitAnswer: (_answer: string) => void
    // Actions passed from parent (where socket is connected)
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

  // No longer instantiating the hook here, as it would create a disconnected socket.
  // const { pointToPlayer, buzzIn, makeDecision } = useOneOfFifteenState()

  // --- Derived State ---

  let myContestant = $derived(
    gameState.contestants.find(
      (c) => c.id === sessionId || c.session_id === sessionId
    )
  )
  let isReady = $derived(myContestant?.ready || false)
  let isActivePlayer = $derived(gameState.active_player_id === sessionId)
  let isEliminated = $derived(myContestant?.eliminated || false)

  // Round Logic
  let isRound1 = $derived(gameState.round === 'round1')
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

  // Helper names
  let activePlayerName = $derived(
    gameState.contestants.find((c) => c.id === gameState.active_player_id)
      ?.name ||
      gameState.active_player_id ||
      'Unknown'
  )
  let pointerName = $derived(activePlayerName) // Alias for clarity
  let deciderName = $derived(activePlayerName) // Alias for clarity

  // Status Message Logic
  let getStatusMessage = () => {
    if (gameState.round === 'lobby') return 'Waiting for game to start...'
    if (isEliminated) return 'ELIMINATED'

    // Round 2
    if (isPointingPhase) {
      if (isMyTurnToPoint) return 'Pick a Player!'
      return `Waiting for ${pointerName} to point...`
    }

    // Round 3
    if (isRound3) {
      if (isBuzzerPhase) return 'BUZZ TO ANSWER!'
      if (isDecisionPhase) return 'Make a Decision!'
      if (isActivePlayer) return 'YOUR TURN!'
      if (gameState.decision_pending) {
        return `${deciderName} is deciding...`
      }
    }

    if (isActivePlayer) return 'YOUR TURN!'

    return `${activePlayerName} is answering...`
  }

  function handleTimeout() {
    if (
      isActivePlayer &&
      (gameState.round === 'round1' || isRound2 || isRound3)
    ) {
      onSubmitAnswer('!!!TIMEOUT!!!')
    }
  }
</script>

<ContestantLayout
  {contestantName}
  statusMessage={getStatusMessage()}
  {isActivePlayer}
  {isEliminated}
  hasPresenter={gameState.has_presenter}
  presenterOnline={gameState.presenter_online}
  score={myContestant?.score || 0}
  lives={myContestant?.lives || 0}
  {isRound1}
  round1Misses={myContestant?.round1_misses}
  hideFooter={gameState.round === 'lobby'}
>
  {#if gameState.round === 'lobby'}
    <LobbyView {isReady} {onToggleReady} />
  {:else if isEliminated}
    <EliminatedView score={myContestant?.score || 0} {isRound3} />
  {:else if gameState.round !== 'finished'}
    <GameView
      {gameState}
      {sessionId}
      {isRound2}
      {isRound3}
      {isPointingPhase}
      {isMyTurnToPoint}
      {isBuzzerPhase}
      {isDecisionPhase}
      {isActivePlayer}
      {pointerName}
      {deciderName}
      {activePlayerName}
      questionNumber={myContestant?.round1_questions !== undefined
        ? myContestant.round1_questions + 1
        : undefined}
      {pointToPlayer}
      {buzzIn}
      {makeDecision}
      onTimeout={handleTimeout}
      {onSubmitAnswer}
    />
  {:else}
    <!-- Finished or other state -->
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
</style>
