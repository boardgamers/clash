<script lang="ts">
  import { Clock, Check } from 'lucide-svelte';
  let { saved, inline = false }: { saved: boolean; inline?: boolean } = $props();
</script>

<i class="early-badge" class:saved class:inline title={saved ? 'Choice saved · Click to edit' : 'Choose ahead'}>
  {#if saved}<Check size={11} style="width:11px;height:11px" aria-label="Choice saved" />{:else}<Clock size={11} style="width:11px;height:11px" aria-label="Choose ahead" />{/if}
</i>

<style>
  .early-badge { position: absolute; top: 3px; right: 3px; display: grid; place-items: center; width: 11px; height: 11px; color: #b87523; }
  .inline { position: static; flex: 0 0 11px; }
  .saved { color: var(--green); }
</style>
