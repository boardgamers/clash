<script lang="ts">
  import { civilizationIcon, civilizationImages as emblems } from './civilization-emblems';
  let { civilization, size = 22 }: { civilization: string; size?: number } = $props();
  const Symbol = $derived(civilizationIcon(civilization));
</script>

{#if emblems[civilization]}
  <span
    aria-hidden="true"
    class="civilization-emblem"
    style={`--emblem:url("${emblems[civilization]}");width:${size}px;height:${size}px`}
  ></span>
{:else}<Symbol {size} aria-hidden="true" />{/if}
