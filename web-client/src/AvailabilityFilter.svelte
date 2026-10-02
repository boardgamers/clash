<script lang="ts">
  import type { Controller } from './controller';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
</script>

<label class="availability-filter">
  <input
    type="checkbox"
    checked={$session.availableOnly}
    onchange={(event) => controller.setAvailableOnly(event.currentTarget.checked)}
  />Available only
</label>
