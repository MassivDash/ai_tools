<script lang="ts">
  import type { GameStateSnapshot } from '../../../hooks/useOneOfFifteenState.svelte'
  import { usePresenterSpeech } from '../../../hooks/usePresenterSpeech.svelte'
  import { generateIntroSpeech } from '../../../api/games/oneOfFifteen/presenterService'

  // Refactored Components
  import PresenterLayout from './presenter/PresenterLayout.svelte'
  import PresenterStage from './presenter/PresenterStage.svelte'
  import PresenterControls from './presenter/PresenterControls.svelte'
  import PresenterContestantList from './presenter/PresenterContestantList.svelte'

  interface Props {
    gameState: GameStateSnapshot
    onStartGame: () => void
    onResetGame: () => void
  }

  let { gameState, onStartGame, onResetGame }: Props = $props()

  // Constants
  const QUESTION_DISPLAY_DURATION_MS = 3000

  // Hooks
  const speech = usePresenterSpeech()

  // State
  let lastSpokenQuestion = $state('')
  let robotEmotion = $state('normal')
  let isIntroPlaying = $state(false)
  let lastSpokenRound = $state('')
  let timeLeft = $state(60)

  let timerInterval: ReturnType<typeof setInterval> | undefined

  // Timer Effect
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

  // Handle Start Logic
  const handleStartGame = async () => {
    if (gameState.contestants.length === 0) return
    isIntroPlaying = true

    // 1. Generate Intro
    const introText = await generateIntroSpeech(gameState.contestants)

    // 2. Speak Intro
    robotEmotion = 'happy'
    await speech.speakAndWait(introText)
    robotEmotion = 'normal'

    // 3. Start Game (Backend)
    isIntroPlaying = false
    onStartGame()
  }

  // Round Announcements Effect
  $effect(() => {
    const announceRound = async () => {
      if (gameState.round !== lastSpokenRound && !isIntroPlaying) {
        if (gameState.round === 'round1') {
          lastSpokenRound = 'round1'
          await speech.speakAndWait('Start of Round 1')
        } else if (gameState.round === 'round2') {
          lastSpokenRound = 'round2'
          await speech.speakAndWait("Let's start Round 2")
        } else if (gameState.round === 'round3') {
          lastSpokenRound = 'round3'
          await speech.speakAndWait('Final Round 3')
        }
      }
    }
    announceRound()
  })

  // Question Speech Effect
  $effect(() => {
    // Only speak question if we are in game
    if (isIntroPlaying || gameState.round === 'lobby') return

    if (
      gameState.current_question &&
      gameState.current_question.text !== lastSpokenQuestion
    ) {
      lastSpokenQuestion = gameState.current_question.text
      robotEmotion = 'happy'
      speech.speak(gameState.current_question.text)
      setTimeout(() => {
        robotEmotion = 'normal'
      }, QUESTION_DISPLAY_DURATION_MS)
    }
  })
</script>

<PresenterLayout>
  {#snippet stage()}
    <PresenterStage
      round={gameState.round}
      currentQuestion={gameState.current_question}
      {robotEmotion}
      robotTalking={speech.robotTalking}
      {timeLeft}
      {isIntroPlaying}
    />
  {/snippet}

  {#snippet controls()}
    <PresenterControls
      round={gameState.round}
      playerCount={gameState.contestants.length}
      {isIntroPlaying}
      onStartGame={handleStartGame}
      {onResetGame}
    />
  {/snippet}

  {#snippet contestants()}
    <PresenterContestantList
      contestants={gameState.contestants}
      round={gameState.round}
    />
  {/snippet}
</PresenterLayout>
