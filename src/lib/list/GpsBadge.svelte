<script lang="ts">
  interface Props {
    hasGps: boolean;
  }
  let { hasGps }: Props = $props();
</script>

<span class="badge" class:has={hasGps}>
  {hasGps ? "HAS GPS" : "MISSING GPS"}
</span>

<style>
  .badge {
    display: inline-block;
    padding: var(--sp-xs) var(--sp-sm);
    border-radius: 3px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.5px;
    background: var(--border);
    color: var(--text-muted);
  }

  .badge.has {
    background: var(--accent);
    color: var(--text-on-accent);
  }
</style>
