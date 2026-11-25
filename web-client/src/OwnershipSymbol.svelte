<script lang="ts">
  let { symbol, size = 12 }: { symbol: string; size?: number } = $props();
</script>

{#if symbol === '●'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <circle cx="8" cy="8" r="5.2" fill="currentColor" />
  </svg>
{:else if symbol === '▲'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <polygon points="8,2.6 13.4,11.9 2.6,11.9" fill="currentColor" />
  </svg>
{:else if symbol === '■'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <rect x="3.4" y="3.4" width="9.2" height="9.2" fill="currentColor" />
  </svg>
{:else if symbol === '◆'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <polygon points="8,2.2 13.8,8 8,13.8 2.2,8" fill="currentColor" />
  </svg>
{:else if symbol === '✚'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <rect x="2.6" y="6.1" width="10.8" height="3.8" fill="currentColor" />
    <rect x="6.1" y="2.6" width="3.8" height="10.8" fill="currentColor" />
  </svg>
{:else if symbol === '✕'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <path
      d="M3.8 3.8L12.2 12.2M12.2 3.8L3.8 12.2"
      stroke="currentColor"
      stroke-width="2.8"
      stroke-linecap="round"
    />
  </svg>
{:else}
  <span class="ownership-symbol-glyph">{symbol}</span>
{/if}
