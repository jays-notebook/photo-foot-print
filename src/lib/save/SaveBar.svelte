<script lang="ts">
  interface Props {
    /** True when pendingPin differs from currentExif OR currentExif is null. */
    enabled: boolean;
    /** True while save_geotag is in flight. */
    saving: boolean;
    /** Click handler — DetailPane orchestrates the dialog + IPC flow. */
    onSave: () => void;
  }
  let { enabled, saving, onSave }: Props = $props();

  // UI-SPEC §Save button states: label switches to "Saving..." while in flight.
  // Three ASCII dots, NOT the unicode ellipsis (UI-SPEC copy invariant).
  let label = $derived(saving ? "Saving..." : "Save");

  // UI-SPEC §Save bar states: hint shown only in the "no pending change" state
  // (i.e. button is disabled because pending matches EXIF AND not saving).
  let showHint = $derived(!enabled && !saving);
</script>

<div class="save-bar">
  {#if showHint}
    <span class="hint">Pin matches saved location.</span>
  {:else}
    <span class="hint" aria-hidden="true"></span>
  {/if}
  <button
    type="button"
    class="primary"
    disabled={!enabled || saving}
    onclick={onSave}
  >
    {label}
  </button>
</div>

<style>
  .save-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-md);
    padding: var(--sp-sm) var(--sp-md);
    height: 56px;
    box-sizing: border-box;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    margin-top: auto; /* pin to bottom of detail-pane flex column */
  }
  .hint {
    color: var(--text-muted);
    font-size: 14px;
  }
  button.primary {
    background: var(--accent);
    color: var(--text-on-accent);
    border: none;
    border-radius: 4px;
    padding: var(--sp-sm) var(--sp-md);
    font-size: 14px;
    font-weight: 400;
    cursor: pointer;
  }
  button.primary:hover:not(:disabled) {
    filter: brightness(0.95);
  }
  button.primary:active:not(:disabled) {
    filter: brightness(0.9);
  }
  button.primary:disabled {
    background: var(--bg-secondary);
    color: var(--text-muted);
    border: 1px solid var(--border);
    cursor: not-allowed;
  }
</style>
