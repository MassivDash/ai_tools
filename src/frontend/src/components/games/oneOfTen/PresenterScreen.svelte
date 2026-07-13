<script lang="ts">
  import type { GameStateSnapshot } from '../../../hooks/useOneOfTenState.svelte'
  import { usePresenterSpeech } from '../../../hooks/usePresenterSpeech.svelte'
  import {
    generateIntroSpeech,
    generateHostJoke,
    generateAnswerComment
  } from '../../../api/games/oneOfTen/presenterService'

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
  let lastFeedbackQuestion = $state('')
  let isSpeakingFeedback = false
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

    // 3. Open with a (not so funny) joke
    const joke = await generateHostJoke()
    robotEmotion = 'surprised'
    await speech.speakAndWait(joke)
    robotEmotion = 'normal'

    // 4. Start Game (Backend)
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

  // Answer Feedback Effect - comments on every submitted answer, correct or wrong,
  // in the window after the round logic clears current_question and before the
  // next question is generated.
  $effect(() => {
    if (isIntroPlaying || gameState.round === 'lobby') return

    const giveFeedback = async () => {
      if (
        !gameState.current_question &&
        gameState.last_answer_correct !== undefined &&
        gameState.last_answer_correct !== null &&
        lastSpokenQuestion &&
        lastSpokenQuestion !== lastFeedbackQuestion
      ) {
        lastFeedbackQuestion = lastSpokenQuestion
        isSpeakingFeedback = true
        robotEmotion = gameState.last_answer_correct ? 'happy' : 'sad'

        const comment = await generateAnswerComment(
          lastSpokenQuestion,
          gameState.last_answer_correct,
          gameState.last_correct_answer
        )
        await speech.speakAndWait(comment)

        robotEmotion = 'normal'
        isSpeakingFeedback = false
      }
    }

    giveFeedback()
  })

  // Question Speech Effect
  $effect(() => {
    // Only speak question if we are in game
    if (isIntroPlaying || gameState.round === 'lobby') return

    const speakQuestionFlow = async () => {
      if (
        gameState.current_question &&
        gameState.current_question.text !== lastSpokenQuestion
      ) {
        lastSpokenQuestion = gameState.current_question.text

        // Let any in-flight answer feedback finish before talking over it
        while (isSpeakingFeedback) {
          await new Promise((r) => setTimeout(r, 100))
        }

        robotEmotion = 'happy'
        speech.speak(gameState.current_question.text)
        setTimeout(() => {
          robotEmotion = 'normal'
        }, QUESTION_DISPLAY_DURATION_MS)
      }
    }

    speakQuestionFlow()
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
