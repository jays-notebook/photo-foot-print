<script lang="ts">
  // Phase 4 (D-56 / D-61, UI-SPEC §7): native datetime-local DTO editor.
  // Pure helpers: toHtml / toExif from ./exifDateTime. The Rust validator
  // (validate_dto_format) is the final authority; this layer is fast
  // feedback only.

  import { toHtml, toExif } from "./exifDateTime";

  interface Props {
    /** EXIF wire format 'YYYY:MM:DD HH:MM:SS' or null. */
    value: string | null;
    /** Emits EXIF wire format or null on every input event. */
    onChange: (next: string | null) => void;
    onValidityChange?: (valid: boolean) => void;
  }
  let { value, onChange, onValidityChange }: Props = $props();

  // Reactive HTML value — re-derives when parent's `value` prop changes
  // (photo switch). Pattern source: ExifReadout.svelte $derived for
  // captureText.
  let html = $derived(toHtml(value));

  function onInput(e: Event): void {
    const target = e.currentTarget as HTMLInputElement;
    // Browsers also expose partial date edits as an empty value. Only a
    // valid empty input means deletion; an incomplete edit blocks saving.
    const valid = target.validity.valid;
    onValidityChange?.(valid);
    if (valid) onChange(toExif(target.value));
  }
</script>

<div class="datetime-editor">
  <label for="dto-input">Capture time</label>
  <input
    id="dto-input"
    type="datetime-local"
    value={html}
    oninput={onInput}
    aria-describedby={value === null && html === "" ? "dto-hint" : undefined}
  />
  <div class="hint-slot">
    {#if value === null && html === ""}
      <span id="dto-hint" class="hint">(no capture time recorded)</span>
    {/if}
  </div>
</div>

<style>
  .datetime-editor label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    letter-spacing: 0.5px;
    margin-bottom: var(--sp-xs);
  }
  .datetime-editor input[type="datetime-local"] {
    width: 100%;
    height: 32px;
    font: 400 14px/1.5 inherit;
    font-variant-numeric: tabular-nums;
    padding: 0 var(--sp-sm);
    background: var(--bg-dominant);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-default);
  }
  .datetime-editor .hint-slot {
    margin-top: var(--sp-xs);
    min-height: 0;
  }
  .datetime-editor .hint {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }
</style>
