<script lang="ts">
  interface Props {
    onBuzzIn: () => void
  }

  let { onBuzzIn }: Props = $props()
</script>

<div class="buzzer-container">
  <button class="buzzer-btn" onclick={onBuzzIn}> BUZZ! </button>
  <p>Be the first to buzz in!</p>
</div>

<style>
  .buzzer-container {
    padding: 3rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .buzzer-btn {
    width: 200px;
    height: 200px;
    border-radius: 50%;
    background: var(--error);
    color: white;
    font-size: 2.5rem;
    font-weight: bold;
    border: 8px solid #c00;
    cursor: pointer;
    box-shadow: 0 4px 15px rgba(0, 0, 0, 0.3);
    transition:
      transform 0.1s,
      box-shadow 0.1s;
  }
  .buzzer-btn:active {
    transform: scale(0.95);
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.3);
  }
</style>
