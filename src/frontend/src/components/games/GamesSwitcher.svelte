<script lang="ts">
  import Card from '../ui/Card.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'

  const games = [
    {
      id: '1-of-15',
      name: '1 of 15',
      description: 'A dedicated quiz game hosted by an AI personality.',
      icon: 'gamepad-variant',
      href: '/games/1-of-15'
    }
    // Future games can be added here
  ]
</script>

<div class="games-switcher">
  <div class="games-header">
    <h2>AI Games</h2>
    <p class="subtitle">Select a game to start playing</p>
  </div>

  <div class="games-grid">
    {#each games as game}
      <a href={game.href} class="game-card-wrapper">
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

  .games-header {
    text-align: center;
    margin-bottom: 3rem;
  }

  .games-header h2 {
    font-size: 2.5rem;
    margin: 0 0 0.5rem 0;
    color: var(--md-on-surface);
  }

  .subtitle {
    font-size: 1.1rem;
    color: var(--md-on-surface-variant);
    margin: 0;
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
