<script lang="ts">
  // Phase 4 (D-48..D-55, UI-SPEC §6): paste-coordinates row.
  // Pure parser delegation: no inline regex; calls parsePasteCoords from
  // ./parsePaste. Inline error wording locked to UI-SPEC §10.1.

  import { parsePasteCoords } from "./parsePaste";

  interface Props {
    /** Fires only with VALID parsed coords. Parent updates pendingPin. */
    onApply: (lat: number, lng: number) => void;
  }
  let { onApply }: Props = $props();

  let raw = $state("");
  let errored = $state(false);
  let canApply = $derived(raw.trim().length > 0);

  function tryApply(): void {
    const result = parsePasteCoords(raw);
    if ("ok" in result) {
      // D-51: input value is NOT cleared on success — user can see what was applied.
      errored = false;
      onApply(result.ok.lat, result.ok.lng);
    } else {
      errored = true;
    }
  }

  function onInput(): void {
    // D-54: error clears on the next keystroke.
    if (errored) errored = false;
  }

  function onKeydown(e: KeyboardEvent): void {
    // D-50: Apply fires on Enter inside the input (equivalent to button click).
    if (e.key === "Enter" && canApply) {
      e.preventDefault();
      tryApply();
    }
  }
</script>

<div class="coord-paste-row">
  <label for="coord-paste-input">Paste coordinates</label>
  <div class="row">
    <input
      id="coord-paste-input"
      type="text"
      bind:value={raw}
      oninput={onInput}
      onkeydown={onKeydown}
      placeholder="35.6586, 139.7454 or Google Maps URL"
      autocomplete="off"
      spellcheck="false"
    />
    <button type="button" disabled={!canApply} onclick={tryApply}>
      Apply
    </button>
  </div>
  <div class="error-slot" aria-live="polite">
    {#if errored}
      <span class="error"
        >Could not parse. Try '35.6586, 139.7454' or a Google Maps URL.</span
      >
    {/if}
  </div>
</div>

<style>
  .coord-paste-row label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    letter-spacing: 0.5px;
    margin-bottom: var(--sp-xs);
  }
  .coord-paste-row .row {
    display: flex;
    gap: var(--sp-sm);
    align-items: stretch;
  }
  .coord-paste-row input[type="text"] {
    flex: 1 1 auto;
    height: 32px;
    font: 400 14px/1.5 inherit;
    font-variant-numeric: tabular-nums;
    padding: 0 var(--sp-sm);
    background: var(--bg-dominant);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-default);
  }
  .coord-paste-row button {
    flex: 0 0 auto;
    height: 32px;
    font: 600 14px/1.5 inherit;
    padding: 0 var(--sp-md);
    background: var(--accent);
    color: var(--text-on-accent);
    border: 1px solid var(--accent);
    border-radius: 4px;
    cursor: pointer;
  }
  .coord-paste-row button:hover:not(:disabled) {
    filter: brightness(0.95);
  }
  .coord-paste-row button:active:not(:disabled) {
    filter: brightness(0.9);
  }
  .coord-paste-row button:disabled {
    background: var(--bg-secondary);
    color: var(--text-muted);
    border-color: var(--border);
    cursor: not-allowed;
  }
  .coord-paste-row .error-slot {
    margin-top: var(--sp-xs);
    min-height: 0;
  }
  .coord-paste-row .error {
    font-size: 12px;
    font-weight: 600;
    color: var(--destructive);
  }
</style>
