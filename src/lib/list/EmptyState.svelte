<script lang="ts">
  interface Props {
    note?: string | null;
    heading?: string;
    body?: string;
  }
  let {
    note = null,
    heading = "Open a folder to start.",
    body = "Pick a folder containing scanner JPEGs. The app reads each photo's EXIF in place — nothing is moved or copied.",
  }: Props = $props();
</script>

<div class="empty">
  <h2>{heading}</h2>
  <p class="body">{body}</p>
  {#if note}
    <p class="note">{note}</p>
  {/if}
</div>

<style>
  .empty {
    padding: var(--sp-lg);
    color: var(--text-muted);
    flex: 1;
  }
  h2 {
    color: var(--text-default);
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 var(--sp-sm) 0;
  }
  .body {
    font-size: 14px;
    margin: 0 0 var(--sp-md) 0;
  }
  .note {
    font-size: 14px;
    font-style: italic;
  }
</style>
