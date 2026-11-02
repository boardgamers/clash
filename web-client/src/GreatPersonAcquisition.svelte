<script lang="ts">
  import ResourceText from './ResourceText.svelte';

  // Keep the complete rule as one localization key; inline resource components
  // otherwise split the second sentence into an untranslated punctuation fragment.
  const acquisitionRule = 'The player who triggered the event may take the card for 1 culture token. If they pass, other players may take it in turn order for 2 culture tokens.';
</script>

<p class="great-person-acquisition"><ResourceText text={acquisitionRule} compactResources /></p>
