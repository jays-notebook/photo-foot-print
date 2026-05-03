<script lang="ts">
  import { openFolderDialog, truncateMiddle, formatError } from "../ipc";

  interface Props {
    folderPath: string | null;
    loading: boolean;
    onOpen: (path: string) => Promise<void>;
    onRefresh: () => Promise<void> | void;
  }

  let { folderPath, loading, onOpen, onRefresh }: Props = $props();

  let opening = $state(false);
  let openError: string | null = $state(null);

  async function handleOpen() {
    opening = true;
    openError = null;
    try {
      const path = await openFolderDialog();
      if (path !== null) {
        await onOpen(path);
      }
    } catch (e) {
      openError = formatError(e);
    } finally {
      opening = false;
    }
  }

  let displayPath = $derived(folderPath ? truncateMiddle(folderPath, 48) : null);
</script>

<header>
  <button
    type="button"
    class="primary"
    onclick={handleOpen}
    disabled={opening || loading}
    aria-label="Open folder"
  >
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      stroke="currentColor"
      stroke-width="1.5"
      fill="none"
      aria-hidden="true"
    >
      <path d="M2 4h4l1 1h7v8H2V4z" />
    </svg>
    <span>Open Folder</span>
  </button>

  {#if displayPath}
    <span class="path" title={folderPath}>{displayPath}</span>
  {/if}

  {#if folderPath}
    <button
      type="button"
      class="icon"
      onclick={() => onRefresh()}
      disabled={loading}
      aria-label="Refresh folder"
      title="Refresh folder"
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 16 16"
        stroke="currentColor"
        stroke-width="1.5"
        fill="none"
        aria-hidden="true"
      >
        <path d="M3 8a5 5 0 0 1 9-3M13 8a5 5 0 0 1-9 3M12 2v3h-3M4 14v-3h3" />
      </svg>
    </button>
  {/if}

  {#if openError}
    <span class="error" role="alert">{openError}</span>
  {/if}
</header>

<style>
  header {
    grid-area: toolbar;
    height: var(--toolbar-h);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: var(--sp-md);
    padding: 0 var(--sp-md);
  }

  button.primary {
    background: var(--accent);
    color: var(--text-on-accent);
    border: none;
    border-radius: 4px;
    padding: var(--sp-sm) var(--sp-md);
    font-size: 14px;
    font-weight: 400;
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    cursor: pointer;
  }

  button.primary:hover {
    filter: brightness(0.95);
  }
  button.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  button.icon {
    background: transparent;
    color: var(--text-default);
    border: none;
    border-radius: 4px;
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    margin-left: auto;
  }

  button.icon:hover {
    background: var(--border);
  }
  button.icon:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .path {
    color: var(--text-muted);
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .error {
    color: var(--destructive);
    font-size: 12px;
  }
</style>
