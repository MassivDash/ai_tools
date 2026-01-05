<script lang="ts">
  let { emotion = $bindable('normal'), talking = $bindable(false) } = $props()

  const gridCols = 32
  const gridRows = 24
  const totalPixels = gridCols * gridRows

  // Helper for internal demo control
  const startTalking = () => {
    let _startTalking = $derived(talking)
    let _prevEmotion = $derived(emotion)
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
          {#each { length: 3 } as _, _i}
            <div class="pixel"></div>
          {/each}
        </div>
        <div class="grid-overlay"></div>
      </div>
    </div>

    <!-- Controls removed as requested, controlled via props now -->
  </div>
</div>

<style lang="scss">
  @import './robot.scss';
</style>
