<script lang="ts">
  import {
    readPhotoMeta,
    saveGeotag,
    getSessionLastPin,
    formatError,
    asWireError,
    type PhotoMeta,
    type PhotoSummary,
    type GpsCoord,
    type CaptureTimeChange,
  } from "../ipc";
  import ExifReadout from "./ExifReadout.svelte";
  import MapPane from "../map/MapPane.svelte";
  import SaveBar from "../save/SaveBar.svelte";
  import { confirmSaveGeotag, showSaveError } from "../save/saveDialog";
  import CoordsPasteRow from "../coords/CoordsPasteRow.svelte";
  import DateTimeEditor from "../datetime/DateTimeEditor.svelte";

  interface Props {
    summary: PhotoSummary | null;
    /** Phase 3: bubble fresh PhotoMeta to parent so the list-row badge flips. */
    onSaved?: (fresh: PhotoMeta) => void;
  }
  let { summary, onSaved }: Props = $props();

  // Default seed: Seoul City Hall (D-26).
  const SEOUL_DEFAULT: GpsCoord = { lat: 37.5665, lng: 126.978 };
  const DEFAULT_ZOOM = 13;

  let meta: PhotoMeta | null = $state(null);
  let loading = $state(false);
  let error: string | null = $state(null);

  let pendingPin: GpsCoord | null = $state(null);
  // Phase 4 (D-61): pendingDto is the editable DTO mirror — null when
  // either the file has no capture_time OR the user cleared the input.
  // Initialised inside the WR-01 cancellation-tokened $effect so a
  // rapid A→B selection sequence cannot land A's pendingDto after B's
  // photo is already showing.
  let pendingDto: string | null = $state(null);
  let dtoValid = $state(true);
  // Informational; live error rendering lives inside CoordsPasteRow.
  // Reserved for cross-component signaling per outline seed 7.
  let pasteError: string | null = $state(null);

  let initialCenter: GpsCoord = $state(SEOUL_DEFAULT);
  let saving = $state(false);
  let selectionVersion = 0;

  $effect(() => {
    const s = summary;
    ++selectionVersion;
    dtoValid = true;
    meta = null;
    pendingDto = null;
    if (!s) {
      loading = false;
      meta = null;
      pendingPin = null;
      pendingDto = null;
      pasteError = null;
      error = null;
      return;
    }
    // WR-01: cancellation token covers pendingPin AND (Phase 4) pendingDto.
    let cancelled = false;
    loading = true;
    error = null;
    void (async () => {
      try {
        const m = await readPhotoMeta(s.id);
        if (cancelled) return;
        const nextCenter: GpsCoord = m.gps
          ? { lat: m.gps.lat, lng: m.gps.lng }
          : ((await getSessionLastPin()) ?? SEOUL_DEFAULT);
        if (cancelled) return;
        meta = m;
        initialCenter = nextCenter;
        pendingPin = { ...initialCenter };
        // Phase 4 (D-61 + UI-SPEC §11.6): pendingDto initial-set is gated
        // by the same `cancelled` flag.
        pendingDto = m.capture_time;
        pasteError = null;
      } catch (e) {
        if (cancelled) return;
        error = formatError(e);
        meta = null;
        pendingPin = null;
        pendingDto = null;
      } finally {
        if (!cancelled) loading = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  // Phase 4 (D-59): split into pinDirty + dtoDirty; saveEnabled is the OR.
  // 6-decimal compare on pinDirty preserved (Pitfall 11 / D-39).
  let pinDirty = $derived.by(() => {
    if (!pendingPin || !meta) return false;
    if (!meta.gps) return true; // first-write case (D-37)
    return (
      pendingPin.lat.toFixed(6) !== meta.gps.lat.toFixed(6) ||
      pendingPin.lng.toFixed(6) !== meta.gps.lng.toFixed(6)
    );
  });
  let dtoDirty = $derived.by(() => {
    const m = meta;
    return pendingDto !== (m ? m.capture_time : null);
  });
  let saveEnabled = $derived.by(() => {
    const current = meta;
    return !loading && dtoValid && current?.id === summary?.id && (pinDirty || dtoDirty);
  });

  // UI-SPEC §11.5: hint widening. Three deterministic strings + the
  // Phase 3 fallback when meta is not yet loaded. Hidden entirely when
  // enabled (SaveBar's own `showHint` derives from `!enabled && !saving`).
  let saveBarHint = $derived.by(() => {
    if (!meta) return null; // SaveBar falls back to Phase 3 default
    if (meta.gps !== null && meta.capture_time !== null) {
      return "Pin and capture time match saved values.";
    }
    if (meta.gps !== null && meta.capture_time === null) {
      return "Pin matches saved location. No capture time recorded.";
    }
    return null; // SaveBar falls back to Phase 3 default
  });

  function onPinChange(lat: number, lng: number) {
    if (!saving && !loading) pendingPin = { lat, lng };
  }

  function onPasteApply(lat: number, lng: number) {
    // UI-SPEC §11.2: paste-applied coords flow into pendingPin; the
    // pendingPin prop on MapPane drives a marker.setLatLng + map.setView.
    if (!saving && !loading) pendingPin = { lat, lng };
  }

  function onDtoChange(next: string | null) {
    if (!saving && !loading) pendingDto = next;
  }

  async function onSave() {
    if (!saveEnabled || saving || !meta || !pendingPin || !summary) return;
    // Freeze the target and values before the native dialog yields control.
    const target = { id: summary.id, fileName: summary.file_name };
    const pin = { ...pendingPin };
    const dto = pendingDto;
    const captureTime: CaptureTimeChange = !dtoDirty
      ? { kind: "keep" }
      : dto === null ? { kind: "remove" } : { kind: "set", value: dto };
    const version = selectionVersion;
    const dialog = {
      fileName: target.fileName,
      newLat: pin.lat,
      newLng: pin.lng,
      oldLat: meta.gps?.lat ?? null,
      oldLng: meta.gps?.lng ?? null,
      newDto: dto,
      oldDto: meta.capture_time,
      dirtyCoords: pinDirty,
      dirtyDto: dtoDirty,
    };
    saving = true;
    try {
      if (!(await confirmSaveGeotag(dialog))) return;
      if (version !== selectionVersion || summary?.id !== target.id) return;
      const fresh = await saveGeotag(target.id, pin.lat, pin.lng, captureTime);
      // A completed save still updates its list row, but never another editor.
      if (version === selectionVersion && summary?.id === target.id) {
        meta = fresh;
        if (fresh.gps) pendingPin = { ...fresh.gps };
        pendingDto = fresh.capture_time;
      }
      onSaved?.(fresh);
    } catch (e) {
      const wire = asWireError(e);
      await showSaveError(target.fileName, wire?.detail ?? formatError(e));
    } finally {
      saving = false;
    }
  }
</script>

{#if !summary}
  <p class="empty">No photo selected.</p>
{:else}
  <h1 class="filename">{summary.file_name}</h1>

  <div class="map-slot" inert={saving || loading}>
    {#if pendingPin}
      <MapPane
        {initialCenter}
        initialZoom={DEFAULT_ZOOM}
        {onPinChange}
        {pendingPin}
      />
    {/if}
  </div>

  {#if loading}
    <p class="muted">Reading EXIF...</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if meta}
    <ExifReadout {meta} />
    <fieldset disabled={saving}>
      <CoordsPasteRow onApply={onPasteApply} />
      <DateTimeEditor value={pendingDto} onChange={onDtoChange} onValidityChange={(valid) => (dtoValid = valid)} />
    </fieldset>
    {#if pasteError}
      <!-- Reserved cross-component signal slot per outline seed 7. -->
      <p class="error">{pasteError}</p>
    {/if}
  {/if}

  <SaveBar enabled={saveEnabled} {saving} {onSave} hint={saveBarHint} />
{/if}

<style>
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
  :global(.detail-pane) {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
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
  .map-slot {
    flex: 1;
    min-height: 320px;
    max-height: calc(100vh - 360px);
    margin-bottom: var(--sp-md);
  }
  .muted {
    color: var(--text-muted);
  }
  .error {
    color: var(--destructive);
  }
</style>
