<script lang="ts">
  import {
    readPhotoMeta,
    formatError,
    type PhotoMeta,
    type PhotoSummary,
  } from "../ipc";
  import ExifReadout from "./ExifReadout.svelte";

  interface Props {
    summary: PhotoSummary | null;
  }
  let { summary }: Props = $props();

  let meta: PhotoMeta | null = $state(null);
  let loading = $state(false);
  let error: string | null = $state(null);

  $effect(() => {
    const s = summary;
    if (!s) {
      meta = null;
      error = null;
      return;
    }
    loading = true;
    error = null;
    void (async () => {
      try {
        meta = await readPhotoMeta(s.id);
      } catch (e) {
        error = formatError(e);
        meta = null;
      } finally {
        loading = false;
      }
    })();
  });
</script>

{#if !summary}
  <p class="empty">No photo selected.</p>
{:else}
  <h1 class="filename">{summary.file_name}</h1>

  <div class="placeholder">
    <svg
      width="48"
      height="48"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      aria-hidden="true"
    >
      <path d="M12 2C8 2 5 5 5 9c0 5 7 13 7 13s7-8 7-13c0-4-3-7-7-7z" />
      <circle cx="12" cy="9" r="2" />
    </svg>
    <p>Map arrives in Phase 3.</p>
  </div>

  {#if loading}
    <p class="muted">Reading EXIF...</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if meta}
    <ExifReadout {meta} />
  {/if}
{/if}

<style>
  .empty {
    text-align: center;
    color: var(--text-muted);
    margin-top: 40vh;
  }
  .filename {
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 var(--sp-md) 0;
    color: var(--text-default);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .placeholder {
    background: var(--bg-secondary);
    border: 1px dashed var(--border);
    border-radius: 6px;
    padding: var(--sp-2xl);
    text-align: center;
    color: var(--text-muted);
    margin-bottom: var(--sp-lg);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-sm);
  }
  .muted {
    color: var(--text-muted);
  }
  .error {
    color: var(--destructive);
  }
</style>
