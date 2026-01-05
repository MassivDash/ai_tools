<script lang="ts">
  import { onMount } from 'svelte'
  import Card from '../ui/Card.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import RobotPresenter from './robot/RobotPresenter.svelte'
  import { useTextToSpeech } from '@hooks/useTextToSpeech.svelte'
  import { useStatusWebSocket } from '@hooks/useStatusWebSocket'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance'

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

  // Interactive Mode
  let isInteractive = $state(false)
  let isLlamaActive = $state(false)
  let serverStarting = $state(false)
  let pendingForceHappy = $state(false)

  const statusWs = useStatusWebSocket((status) => {
    isLlamaActive = status.active
    if (!status.active) isInteractive = false
  })

  // LLM Helper
  const generateRobotSpeech = async (prompt: string): Promise<string> => {
    try {
      // Don't animate while thinking, just wait for text
      // robotTalking = true

      const systemPrompt = `You are a quirky, slightly emotional Robot Quiz Host named Quiz Bot. 
      Keep your response extremely short (under 15 words). 
      Respond to the following situation:`

      const requestPayload = {
        message: `${systemPrompt} ${prompt}`,
        conversation_id: undefined
      }

      /* eslint-disable no-undef */
      const response = await fetch(
        `${axiosBackendInstance.defaults.baseURL}/agent/chat/stream`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(requestPayload)
        }
      )
      /* eslint-enable no-undef */

      if (!response.body) return 'Error: No response'

      const reader = response.body.getReader()
      /* eslint-disable no-undef */
      const decoder = new TextDecoder()
      /* eslint-enable no-undef */
      let fullText = ''

      while (true) {
        const { done, value } = await reader.read()
        if (done) break
        const chunk = decoder.decode(value)
        const lines = chunk.split('\n')
        for (const line of lines) {
          if (line.startsWith('data: ')) {
            try {
              const data = JSON.parse(line.slice(6))
              if (data.type === 'text_chunk' && data.text) {
                fullText += data.text
              }
            } catch (_e) {
              // ignore
            }
          }
        }
      }
      return fullText.trim() || '...'
    } catch (e) {
      console.error('LLM Error', e)
      return 'I am having trouble thinking...'
    }
  }

  const tts = useTextToSpeech({ rate: 1.0, pitch: 1.0, volume: 1.0 })

  // Sync Animation with Sound
  let wasSpeaking = false
  $effect(() => {
    if (soundOn) {
      if (tts.isSpeaking) {
        if (!robotTalking) robotTalking = true // Force animation ON if speaking
        wasSpeaking = true
      } else if (wasSpeaking) {
        // Just finished speaking
        robotTalking = false
        if (pendingForceHappy) {
          robotEmotion = 'happy'
          pendingForceHappy = false
        }
        wasSpeaking = false
      }
    }
  })

  // Helper to Speak and Wait for completion
  const speakAndWait = async (text: string, forceHappy = false) => {
    robotMessage = text
    robotTalking = true

    if (soundOn) {
      pendingForceHappy = forceHappy
      tts.speak(text)

      // Wait until tts starts speaking (short buffer)
      await new Promise((r) => setTimeout(r, 100))

      // Poll until speaking is done
      while (tts.isSpeaking) {
        await new Promise((r) => setTimeout(r, 200))
        if (!soundOn) break // break if user toggles off
      }
    } else {
      // Fallback duration if sound is off
      const duration = text.length * 60 + 1000
      await new Promise((r) => setTimeout(r, duration))
      robotTalking = false
      if (forceHappy) robotEmotion = 'happy'
    }
  }

  const toggleSound = () => {
    soundOn = !soundOn
    if (soundOn) {
      if (robotMessage) {
        tts.speak(robotMessage)
        robotTalking = true
      }
    } else {
      tts.cancel()
      robotTalking = false
    }
  }

  const startLlamaServer = async () => {
    try {
      serverStarting = true
      await axiosBackendInstance.post('llama-server/start')
    } catch (e) {
      console.error('Failed to start server', e)
    } finally {
      serverStarting = false
    }
  }

  const toggleInteractive = async () => {
    if (!isLlamaActive) {
      await startLlamaServer()
      return
    }
    isInteractive = !isInteractive
  }

  onMount(() => {
    statusWs.connect()

    const runSequence = async () => {
      if (hasInteracted) return

      // 1. Start Neutral (No bubble)
      robotMessage = ''
      robotEmotion = 'normal'
      robotTalking = false
      await new Promise((r) => setTimeout(r, 1000))
      if (hasInteracted) return

      // 2. Happy (Wait 1s)
      robotEmotion = 'happy'
      await new Promise((r) => setTimeout(r, 1000))
      if (hasInteracted) return

      // 3. Welcome
      let welcomeText = 'Welcome to games in Meduza, I am quiz bot'
      if (isInteractive && isLlamaActive) {
        welcomeText = await generateRobotSpeech(
          'Greet the user warmly and invite them to play a game.'
        )
      }

      // Bubble + Talk
      robotEmotion = 'talking'
      await speakAndWait(welcomeText)

      // 4. Wait 3s and Stop Bubble
      await new Promise((r) => setTimeout(r, 3000))
      robotMessage = '' // hide bubble
      if (hasInteracted) return

      // 5. Wait 2s
      robotTalking = false // ensure stop
      robotEmotion = 'normal'
      await new Promise((r) => setTimeout(r, 2000))
      if (hasInteracted) return

      // 6. Sad
      robotEmotion = 'sad'
      await new Promise((r) => setTimeout(r, 1000))
      if (hasInteracted) return

      // 7. Nag (Sad) - text + talk
      let nagText = 'Why are you not playing?'
      if (isInteractive && isLlamaActive) {
        nagText = await generateRobotSpeech(
          'You are sad. Ask the user why they are ignoring you.'
        )
      }
      // Ensure sad emotion persists during talk
      robotEmotion = 'sad'
      await speakAndWait(nagText)

      // Clear bubble after 1s
      await new Promise((r) => setTimeout(r, 1000))
      robotMessage = ''
      if (hasInteracted) return

      // 8. Wait 2s
      await new Promise((r) => setTimeout(r, 2000))
      if (hasInteracted) return

      // 9. Angry
      robotEmotion = 'angry'
      await new Promise((r) => setTimeout(r, 1000))
      if (hasInteracted) return

      // 10. Shout
      let shoutText = "Come on, let's play!!!"
      if (isInteractive && isLlamaActive) {
        shoutText = await generateRobotSpeech(
          'You are angry and impatient. Shout at the user to start playing!'
        )
      }
      robotEmotion = 'angry'
      await speakAndWait(shoutText)

      // Clear bubble after 1s
      await new Promise((r) => setTimeout(r, 1000))
      robotMessage = ''
      if (hasInteracted) return

      // 11. Repeat after 15s
      robotEmotion = 'normal'
      await new Promise((r) => setTimeout(r, 15000))
      if (!hasInteracted) runSequence()
    }

    runSequence()

    return () => {
      tts.cancel()
      statusWs.disconnect()
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
    </div>

    <!-- Controls Row -->
    <div class="robot-controls-row">
      <button
        class="control-btn sound-toggle"
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

      <button
        class="control-btn interactive-toggle"
        class:active={isInteractive}
        class:server-off={!isLlamaActive}
        onclick={toggleInteractive}
        title={isLlamaActive
          ? isInteractive
            ? 'Disable AI improv'
            : 'Enable AI improv'
          : 'Start AI Server'}
        disabled={serverStarting}
      >
        <MaterialIcon
          name={serverStarting ? 'loading' : 'brain'}
          width="24"
          height="24"
          class={serverStarting ? 'spin' : ''}
        />
        {#if !serverStarting}
          {#if !isLlamaActive}
            <div class="params-badge off">OFF</div>
          {:else if isInteractive}
            <div class="params-badge on">AI</div>
          {/if}
        {/if}
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
    position: relative; /* Context for absolute positioning if needed */
  }

  /* Robot Scaling */
  .robot-container-scaled {
    position: relative;
    width: 320px;
    height: 240px;
    margin: 100px auto 0;
  }

  .scale-wrapper {
    transform: scale(0.5);
    transform-origin: top left;
    width: 640px;
    height: 480px;
    pointer-events: none;
  }

  .title-section {
    text-align: center;
    margin-top: 0;
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

  /* Robot Controls Row */
  .robot-controls-row {
    display: flex;
    flex-direction: row;
    gap: 1.5rem;
    margin-top: 4rem; /* Increased top margin as requested */
    z-index: 5;
    margin-bottom: 2rem;
    justify-content: center;
    pointer-events: auto;
  }

  .control-btn {
    background: var(--md-surface-container-high);
    color: var(--md-on-surface);
    border: none;
    border-radius: 50%;
    width: 56px; /* Slightly larger targets */
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
    position: relative;
  }

  .control-btn:hover {
    background: var(--md-primary);
    color: var(--md-on-primary);
    transform: scale(1.1);
  }

  .control-btn.active {
    background: var(--md-primary);
    color: var(--md-on-primary);
  }

  .control-btn.server-off {
    border: 3px solid var(--md-error, #cf9292);
  }

  .params-badge {
    position: absolute;
    top: -5px;
    right: -10px;
    font-size: 0.75rem;
    padding: 2px 6px;
    border-radius: 10px;
    font-weight: bold;
    pointer-events: none;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  }

  .params-badge.on {
    background: #44ff44;
    color: black;
  }

  .params-badge.off {
    background: #ff4444;
    color: white;
  }

  /* Games Grid */
  .games-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 2rem;
    margin-top: 2rem;
  }

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

  /* Spin Animation */
  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
