<script lang="ts">
  let { symbol, size = 12 }: { symbol: string; size?: number } = $props();
</script>

{#if symbol === '●'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <circle cx="8" cy="8" r="6.2" fill="currentColor" />
  </svg>
{:else if symbol === '▲'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <polygon points="8,2 14.2,13.2 1.8,13.2" fill="currentColor" />
  </svg>
{:else if symbol === '■'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <rect x="2.2" y="2.2" width="11.6" height="11.6" fill="currentColor" />
  </svg>
{:else if symbol === '◆'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <polygon points="8,1.4 14.6,8 8,14.6 1.4,8" fill="currentColor" />
  </svg>
{:else if symbol === '✚'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <rect x="1.8" y="5.8" width="12.4" height="4.4" fill="currentColor" />
    <rect x="5.8" y="1.8" width="4.4" height="12.4" fill="currentColor" />
  </svg>
{:else if symbol === '✕'}
  <svg class="ownership-symbol-svg" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <path
      d="M3 3L13 13M13 3L3 13"
      stroke="currentColor"
      stroke-width="3.2"
      stroke-linecap="round"
    />
  </svg>
{:else}
  <span class="ownership-symbol-glyph">{symbol}</span>
{/if}
