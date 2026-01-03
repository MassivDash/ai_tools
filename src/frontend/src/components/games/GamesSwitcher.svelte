<script lang="ts">
  import { onMount } from 'svelte'
  import Card from '../ui/Card.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import RobotPresenter from './robot/RobotPresenter.svelte'
  import { useTextToSpeech } from '../../hooks/useTextToSpeech.svelte'

  const games = [
    {
      id: '1-of-15',
      name: '1 of 15',
      description: 'A dedicated quiz game hosted by an AI personality.',
      icon: 'gamepad-variant',
      href: '/games/1-of-15'
    }
  ]

  // Robot State
  let robotEmotion = $state('normal')
  let robotTalking = $state(false)
  let robotMessage = $state('')
  let soundOn = $state(false) // Default off
  let hasInteracted = $state(false)

  const tts = useTextToSpeech({ rate: 1.0, pitch: 1.0, volume: 1.0 })

  const speak = (text: string, forceHappy = false) => {
    robotMessage = text

    // Always animate talking
    robotTalking = true

    // Play sound only if enabled
    if (soundOn) {
      tts.speak(text)
    }

    // Stop "talking" animation after duration (independent of audio)
    const duration = text.length * 60 + 1000
    setTimeout(() => {
      robotTalking = false
      if (forceHappy) robotEmotion = 'happy'
    }, duration)
  }

  const toggleSound = () => {
    soundOn = !soundOn
    // toggling sound should not stop the sequence (hasInteracted remains false)

    if (soundOn) {
      // If we have a message, speak it now that sound is enabled
      if (robotMessage) {
        tts.speak(robotMessage)
      }
    } else {
      tts.cancel()
    }
  }

  onMount(() => {
    let sequenceTimeout: ReturnType<typeof setTimeout>
    let loopInterval: ReturnType<typeof setTimeout>

    const runSequence = async () => {
      if (hasInteracted) return

      // Step 1: Happy for 1s
      robotEmotion = 'happy'
      await new Promise((r) => setTimeout(r, 1000))
      if (hasInteracted) return

      // Step 2: Talk Welcome (2s approx)
      robotEmotion = 'talking'
      speak('Welcome to games in Meduza, I am quiz bot') // Corrected "Weloce"
      await new Promise((r) => setTimeout(r, 3000)) // Wait for talk to finish (2s + buffer)
      if (hasInteracted) return

      // Step 3: Normal for 2s
      robotEmotion = 'normal'
      robotTalking = false
      await new Promise((r) => setTimeout(r, 2000))
      if (hasInteracted) return

      // Step 4: Sad for 2s
      robotEmotion = 'sad'
      await new Promise((r) => setTimeout(r, 2000))
      if (hasInteracted) return

      // Step 5: Talk Nag (still sad)
      // "why not you ar enot playing" -> "Why are you not playing?"
      robotEmotion = 'sad' // Ensure sad
      robotTalking = true // Manually trigger talking animation if sound on or off?
      // If sound off, we usually just set emotion. But user wants talking animation + sad.
      // My component handles `talking` prop + `emotion` class.
      // speak() sets `robotTalking = true` if soundOn.
      // If sound is OFF, we should still animate talking if user wants it?
      // User said "then talk". I will force talking animation briefly even if sound off for this sequence?
      // Actually, let's stick to speak() logic for consistency, but maybe force talking=true manually.

      robotMessage = 'Why are you not playing?'
      if (soundOn) tts.speak(robotMessage)
      robotTalking = true

      await new Promise((r) => setTimeout(r, 2000))
      robotTalking = false
      if (hasInteracted) return

      // Step 6: Wait 2s (still sad?)
      await new Promise((r) => setTimeout(r, 2000))
      if (hasInteracted) return

      // Step 7: Angry for 2s
      robotEmotion = 'angry'
      await new Promise((r) => setTimeout(r, 2000))
      if (hasInteracted) return

      // Step 8: Talk Shout
      // "Say Come one lets Play!!!" -> "Come on, let's play!!!"
      robotMessage = "Come on, let's play!!!"
      if (soundOn) tts.speak(robotMessage)
      robotTalking = true

      await new Promise((r) => setTimeout(r, 2000))
      robotTalking = false
      if (hasInteracted) return

      // Step 9: Wait 5s then repeat
      await new Promise((r) => setTimeout(r, 5000))
      if (!hasInteracted) {
        runSequence() // Recursive repeat
      }
    }

    runSequence()

    return () => {
      // formatting:off
      // Clean up is handled by checking hasInteracted or simple timeouts,
      // but strictly we should clear valid timeouts.
      // Since we use await with local state checks, it's safer.
      // But we must ensure audio stops.
      tts.cancel()
    }
  })

  const onGameSelect = () => {
    hasInteracted = true
    robotEmotion = 'happy'
    robotTalking = false
    robotMessage = ''
    tts.cancel()
  }
</script>

<div class="games-switcher">
  <div class="header-section">
    <div class="robot-container-scaled">
      <div class="speech-bubble" class:visible={!!robotMessage}>
        {robotMessage}
      </div>
      <div class="scale-wrapper">
        <RobotPresenter
          bind:emotion={robotEmotion}
          bind:talking={robotTalking}
        />
      </div>
      <button
        class="sound-toggle"
        class:active={soundOn}
        onclick={toggleSound}
        title={soundOn ? 'Mute Sound' : 'Enable Sound'}
      >
        <MaterialIcon
          name={soundOn ? 'volume-high' : 'volume-off'}
          width="24"
          height="24"
        />
      </button>
    </div>

    <div class="title-section">
      <h2>AI Games</h2>
      <p class="subtitle">Select a game to start playing</p>
    </div>
  </div>

  <div class="games-grid">
    {#each games as game}
      <a href={game.href} class="game-card-wrapper" onclick={onGameSelect}>
        <Card class="game-card-content" variant="elevated">
          <div class="game-icon-wrapper">
            <MaterialIcon name={game.icon} width="48" height="48" />
          </div>
          <h3 class="game-name">{game.name}</h3>
          <p class="game-description">{game.description}</p>
        </Card>
      </a>
    {/each}
  </div>
</div>

<style>
  .games-switcher {
    width: 100%;
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem 1rem;
    margin-bottom: 5rem;
  }

  .header-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 3rem;
    gap: 1rem;
  }

  /* Robot Scaling */
  .robot-container-scaled {
    position: relative;
    width: 320px; /* Scaled width (640 * 0.5) */
    height: 240px; /* Scaled height (480 * 0.5) */
    margin: 100px auto 0; /* Add top margin for bubble visibility */
  }

  .scale-wrapper {
    transform: scale(0.5);
    transform-origin: top left;
    width: 640px;
    height: 480px;
    /* Hide internal robot background if needed, or rely on robot styles */
    pointer-events: none; /* Let clicks pass through if needed, though buttons are internal */
  }

  /* Force pointer events on for the internal controls if we wanted them, 
     but here we overlay our own controls or just use it for display */

  .title-section {
    text-align: center;
    margin-top: -20px; /* Pull up closer to robot */
  }

  h2 {
    font-size: 2.5rem;
    margin: 0 0 0.5rem 0;
    color: var(--md-on-surface);
  }

  .subtitle {
    font-size: 1.1rem;
    color: var(--md-on-surface-variant);
    margin: 0;
  }

  /* Speech Bubble */
  .speech-bubble {
    position: absolute;
    top: -60px;
    left: 50%;
    transform: translateX(-50%) scale(0.8);
    background: white;
    color: black;
    padding: 1rem 1.5rem;
    border-radius: 20px;
    font-family: 'Comic Sans MS', 'Chalkboard SE', sans-serif;
    font-size: 1.1rem;
    border: 3px solid black;
    box-shadow: 4px 4px 0 rgba(0, 0, 0, 0.2);
    z-index: 10;
    opacity: 0;
    transition:
      opacity 0.3s,
      transform 0.3s;
    pointer-events: none;
    white-space: nowrap;
    max-width: 400px;
    text-overflow: ellipsis;
    overflow: hidden;
  }

  .speech-bubble.visible {
    opacity: 1;
    transform: translateX(-50%) scale(1);
  }

  .speech-bubble::after {
    content: '';
    position: absolute;
    bottom: -15px;
    left: 50%;
    transform: translateX(-50%);
    border-width: 15px 15px 0;
    border-style: solid;
    border-color: black transparent transparent transparent;
  }

  .speech-bubble::before {
    content: '';
    position: absolute;
    bottom: -10px;
    left: 50%;
    transform: translateX(-50%);
    border-width: 13px 13px 0;
    border-style: solid;
    border-color: white transparent transparent transparent;
    z-index: 1;
  }

  /* Sound Toggle */
  .sound-toggle {
    position: absolute;
    bottom: 0;
    right: -40px;
    background: var(--md-surface-container-high);
    color: var(--md-on-surface);
    border: none;
    border-radius: 50%;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .sound-toggle:hover {
    background: var(--md-primary);
    color: var(--md-on-primary);
  }

  .sound-toggle.active {
    background: var(--md-primary);
    color: var(--md-on-primary);
  }

  .games-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 2rem;
    margin-top: 2rem;
  }

  /* Interactive wrapper for Card */
  .game-card-wrapper {
    text-decoration: none;
    color: inherit;
    display: flex;
    transition: transform 0.2s;
  }

  .game-card-wrapper:hover {
    transform: translateY(-4px);
  }

  .game-card-wrapper:hover :global(.card) {
    box-shadow: 0 8px 16px -4px var(--md-shadow, rgba(0, 0, 0, 0.1));
    border-color: var(--md-primary);
  }

  :global(.game-card-content) {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 1rem;
    justify-content: center;
    min-height: 250px;
    padding: 2rem;
  }

  .game-icon-wrapper {
    color: var(--md-primary);
    margin-bottom: 0.5rem;
    padding: 1rem;
    background-color: var(--md-primary-container);
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .game-name {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 500;
    color: var(--md-on-surface);
  }

  .game-description {
    margin: 0;
    color: var(--md-on-surface-variant);
    font-size: 1rem;
    line-height: 1.5;
  }
</style>
