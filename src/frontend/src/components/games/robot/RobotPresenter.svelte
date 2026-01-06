<script lang="ts">
  let { emotion = $bindable('normal'), talking = $bindable(false) } = $props()

  const gridCols = 32
  const gridRows = 24
  const totalPixels = gridCols * gridRows

  // Helper for internal demo control removed as it is unused and confusing logic
</script>

<div class="robot-wrapper">
  <div class="robot-container">
    <div class="robot-head {emotion} {talking ? 'talking' : ''}">
      <div class="screen">
        <div class="pixels">
          {#each { length: totalPixels } as _, _i}
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
