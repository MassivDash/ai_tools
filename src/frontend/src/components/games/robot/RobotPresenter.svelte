<script lang="ts">
  let { emotion = $bindable('normal'), talking = $bindable(false) } = $props()

  const gridCols = 32
  const gridRows = 24
  const totalPixels = gridCols * gridRows

  // Helper for internal demo control
  const startTalking = () => {
    if (talking) return
    talking = true
    const prevEmotion = emotion
    emotion = 'talking'
    setTimeout(() => {
      talking = false
      emotion = 'normal' // Reset to normal as per original React logic
    }, 3000) // 3 seconds as per original
  }
</script>

<div class="robot-wrapper">
  <div class="robot-container">
    <div class="robot-head {emotion} {talking ? 'talking' : ''}">
      <div class="screen">
        <div class="pixels">
          {#each { length: totalPixels } as _, i}
            <div class="pixel"></div>
          {/each}
        </div>
        <div class="grid-overlay"></div>
      </div>
    </div>

    <!-- Controls for demo/testing -->
    <div class="controls">
      <button onclick={() => (emotion = 'happy')}>Happy</button>
      <button onclick={() => (emotion = 'normal')}>Normal</button>
      <button onclick={() => (emotion = 'sad')}>Sad</button>
      <button onclick={() => (emotion = 'surprised')}>Surprised</button>
      <button onclick={() => (emotion = 'angry')}>Angry</button>
      <button onclick={startTalking}>Talk</button>
    </div>
  </div>
</div>

<style lang="scss">
  @import './robot.scss';
</style>
