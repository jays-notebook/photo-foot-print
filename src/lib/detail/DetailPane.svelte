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
  } from "../ipc";
  import ExifReadout from "./ExifReadout.svelte";
  import MapPane from "../map/MapPane.svelte";
  import SaveBar from "../save/SaveBar.svelte";
  import { confirmSaveGeotag, showSaveError } from "../save/saveDialog";

  interface Props {
    summary: PhotoSummary | null;
    /** Phase 3: bubble fresh PhotoMeta to parent so the list-row badge flips. */
    onSaved?: (fresh: PhotoMeta) => void;
  }
  let { summary, onSaved }: Props = $props();

  // Default seed: Seoul City Hall (D-26). Used when there is no EXIF GPS
  // AND no session_last_pin yet.
  const SEOUL_DEFAULT: GpsCoord = { lat: 37.5665, lng: 126.978 };
  const DEFAULT_ZOOM = 13;

  let meta: PhotoMeta | null = $state(null);
  let loading = $state(false);
  let error: string | null = $state(null);

  // Pending pin (UX-only state — never written to disk until Save clicked).
  let pendingPin: GpsCoord | null = $state(null);
  // Initial map center for the currently-selected photo. Recomputed every
  // time `meta` changes.
  let initialCenter: GpsCoord = $state(SEOUL_DEFAULT);
  let saving = $state(false);

  $effect(() => {
    const s = summary;
    if (!s) {
      meta = null;
      pendingPin = null;
      error = null;
      return;
    }
    // WR-01: cancellation token for out-of-order resolves. Without this,
    // a rapid A->B selection sequence can land like
    //   - B's IIFE starts, sets meta = null
    //   - B's readPhotoMeta resolves, sets meta = mB
    //   - A's still-pending readPhotoMeta finally resolves, overwrites
    //     meta = mA -- visible filename header reads B (driven by
    //     `summary.file_name`, which is current) but the EXIF readout,
    //     map center, and pin all reflect A.
    // Setting `cancelled = true` on effect re-run / teardown causes the
    // older closure to short-circuit before any `meta = ...` assignment.
    let cancelled = false;
    loading = true;
    error = null;
    void (async () => {
      try {
        const m = await readPhotoMeta(s.id);
        if (cancelled) return;
        // D-25 / D-26 / D-27 initial center fallback chain:
        //   1. has-GPS  → photo coords
        //   2. no-GPS   + session_last_pin → last-saved coords
        //   3. no-GPS   + no last-pin     → Seoul default
        const nextCenter: GpsCoord = m.gps
          ? { lat: m.gps.lat, lng: m.gps.lng }
          : ((await getSessionLastPin()) ?? SEOUL_DEFAULT);
        if (cancelled) return;
        meta = m;
        initialCenter = nextCenter;
        // pendingPin starts at the initial center.
        pendingPin = { ...initialCenter };
      } catch (e) {
        if (cancelled) return;
        error = formatError(e);
        meta = null;
        pendingPin = null;
      } finally {
        if (!cancelled) loading = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  // SaveBar.enabled: pendingPin differs from EXIF GPS (or there is no EXIF GPS).
  // Compared at 6-decimal precision per D-39 / Pitfall 11.
  let saveEnabled = $derived.by(() => {
    if (!pendingPin || !meta) return false;
    if (!meta.gps) return true; // first-write case (D-37)
    return (
      pendingPin.lat.toFixed(6) !== meta.gps.lat.toFixed(6) ||
      pendingPin.lng.toFixed(6) !== meta.gps.lng.toFixed(6)
    );
  });

  function onPinChange(lat: number, lng: number) {
    pendingPin = { lat, lng };
  }

  async function onSave() {
    if (!meta || !pendingPin || !summary) return;
    const confirmed = await confirmSaveGeotag({
      fileName: summary.file_name,
      newLat: pendingPin.lat,
      newLng: pendingPin.lng,
      oldLat: meta.gps?.lat ?? null,
      oldLng: meta.gps?.lng ?? null,
    });
    if (!confirmed) return;
    saving = true;
    try {
      // eslint-disable-next-line prettier/prettier -- single-line for plan literal-grep
      const fresh = await saveGeotag(summary.id, pendingPin.lat, pendingPin.lng);
      // D-42: silent post-save feedback. Replace meta locally; bubble
      // fresh PhotoMeta up so the parent updates listing.items[i] and
      // the row's GpsBadge re-derives via Svelte reactivity (EXIF-10).
      meta = fresh;
      // pendingPin now matches fresh.gps → SaveBar disables.
      if (fresh.gps) {
        pendingPin = { lat: fresh.gps.lat, lng: fresh.gps.lng };
      }
      onSaved?.(fresh);
    } catch (e) {
      // D-41: native error dialog; keep pendingPin so user can retry.
      const wire = asWireError(e);
      await showSaveError(
        summary.file_name,
        wire?.detail ?? formatError(e),
      );
    } finally {
      saving = false;
    }
  }
</script>

{#if !summary}
  <p class="empty">No photo selected.</p>
{:else}
  <h1 class="filename">{summary.file_name}</h1>

  <div class="map-slot">
    {#if pendingPin}
      <MapPane
        {initialCenter}
        initialZoom={DEFAULT_ZOOM}
        {onPinChange}
      />
    {/if}
  </div>

  {#if loading}
    <p class="muted">Reading EXIF...</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if meta}
    <ExifReadout {meta} />
  {/if}

  <SaveBar enabled={saveEnabled} {saving} {onSave} />
{/if}

<style>
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
