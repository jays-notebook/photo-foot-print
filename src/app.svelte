<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    getAppState,
    listFolder,
    onThumbnailReady,
    onThumbnailFailed,
    type FolderListing,
    type PhotoMeta,
    type PhotoSummary,
    formatError,
  } from "./lib/ipc";
  import Toolbar from "./lib/toolbar/Toolbar.svelte";
  import PhotoList from "./lib/list/PhotoList.svelte";
  import DetailPane from "./lib/detail/DetailPane.svelte";
  import EmptyState from "./lib/list/EmptyState.svelte";
  import ThumbCacheBanner from "./lib/list/ThumbCacheBanner.svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  // ----- Reactive state -----
  let listing = $state<FolderListing | null>(null);
  let loading = $state(false);
  let bootstrapError = $state<string | null>(null);
  let selectedId = $state<string | null>(null);
  // EmptyState message — null = first launch, string = "previous folder not found: {path}"
  let emptyStateNote = $state<string | null>(null);
  // Banner dismissal lasts for the session. Reset on every successful load
  // where thumb_cache_writable is false again (per UI-SPEC).
  let bannerDismissed = $state(false);
  // Thumbnail readiness map: id -> reload counter (force <img> re-fetch when set).
  let thumbReady: Record<string, number> = $state({});
  // WR-06: per-id failure tick. PhotoList watches this and drops failed
  // ids from its in-flight `requested` set so the IntersectionObserver
  // can retry on next viewport entry (transient USB / quota recovery).
  let thumbFailed: Record<string, number> = $state({});

  let unlistenThumbReady: UnlistenFn | null = null;
  let unlistenThumbFailed: UnlistenFn | null = null;

  // ----- Bootstrap (D-13) -----
  onMount(async () => {
    try {
      const appState = await getAppState();
      const lf = appState.last_folder;
      if (lf.kind === "available") {
        await loadFolder(lf.path);
      } else if (lf.kind === "missing") {
        emptyStateNote = `Previous folder not found: ${lf.path}`;
      }
      // kind: "none" -> no note, just first-launch EmptyState.
    } catch (e) {
      bootstrapError = `Bootstrap failed: ${formatError(e)}`;
    }

    // WR-09: defensively detach any prior listener before registering a
    // fresh one. In production `onMount` runs once, but Vite HMR
    // re-runs the script block in dev and would otherwise leak handlers
    // that keep mutating `thumbReady` / `thumbFailed` for ids belonging
    // to a previous folder load.
    if (unlistenThumbReady) {
      unlistenThumbReady();
      unlistenThumbReady = null;
    }
    if (unlistenThumbFailed) {
      unlistenThumbFailed();
      unlistenThumbFailed = null;
    }
    unlistenThumbReady = await onThumbnailReady((id) => {
      // Increment a reload counter so PhotoRow's <img src> remounts.
      thumbReady = { ...thumbReady, [id]: (thumbReady[id] ?? 0) + 1 };
    });
    unlistenThumbFailed = await onThumbnailFailed((id, reason) => {
      console.warn("[thumbnail-failed]", id, reason);
      // Bump the per-id tick. PhotoList's $effect on `thumbFailed`
      // removes the id from its `requested` set and re-observes the row,
      // so the next viewport entry triggers a retry.
      thumbFailed = { ...thumbFailed, [id]: (thumbFailed[id] ?? 0) + 1 };
    });
  });

  onDestroy(() => {
    if (unlistenThumbReady) unlistenThumbReady();
    if (unlistenThumbFailed) unlistenThumbFailed();
  });

  // ----- Folder load (called from Toolbar) -----
  async function loadFolder(path: string) {
    loading = true;
    bootstrapError = null;
    emptyStateNote = null;
    selectedId = null;
    bannerDismissed = false;
    try {
      listing = await listFolder(path);
    } catch (e) {
      bootstrapError = formatError(e);
      listing = null;
    } finally {
      loading = false;
    }
  }

  // ----- Selection sync -----
  let selectedSummary = $derived<PhotoSummary | null>(
    listing?.items.find((i: PhotoSummary) => i.id === selectedId) ?? null,
  );

  // ----- Phase 3: post-save list-row update (EXIF-10) -----
  // DetailPane bubbles a fresh PhotoMeta up after a successful
  // save_geotag IPC call. Replace the corresponding row's
  // PhotoSummary in place so PhotoRow's `summary.has_gps`
  // derivation flips MISSING GPS -> HAS GPS without a manual refresh.
  function handleSavedPhoto(fresh: PhotoMeta) {
    if (!listing) return;
    const i = listing.items.findIndex((s) => s.id === fresh.id);
    if (i === -1) return;
    // Replace in place; Svelte 5 deep reactivity picks up the assignment.
    // PhotoSummary.has_gps is server-derived; recompute by checking
    // whether the freshly re-read EXIF surfaced GPS coordinates.
    listing.items[i] = {
      ...listing.items[i],
      has_gps: fresh.gps !== null,
      capture_time: fresh.capture_time,
    };
  }

  // ----- Banner visibility -----
  let showBanner = $derived(
    listing !== null && !listing.thumb_cache_writable && !bannerDismissed,
  );
</script>

<Toolbar
  folderPath={listing?.folder_path ?? null}
  {loading}
  onOpen={loadFolder}
  onRefresh={() => {
    if (listing) void loadFolder(listing.folder_path);
  }}
/>

<div class="list-pane">
  {#if showBanner}
    <ThumbCacheBanner ondismiss={() => (bannerDismissed = true)} />
  {/if}

  {#if bootstrapError}
    <p class="error">{bootstrapError}</p>
  {:else if !listing && !loading}
    <EmptyState note={emptyStateNote} />
  {:else if listing && listing.items.length === 0 && !loading}
    <EmptyState
      note={null}
      heading="No JPEGs in this folder."
      body="This folder has no readable JPEGs. Pick a different folder, or add files and refresh."
    />
  {:else if listing}
    <PhotoList
      items={listing.items}
      footer={listing.footer}
      {selectedId}
      {thumbReady}
      {thumbFailed}
      {loading}
      onSelect={(id) => (selectedId = id)}
    />
  {:else}
    <p class="loading">Loading...</p>
  {/if}
</div>

<div class="detail-pane">
  <DetailPane summary={selectedSummary} onSaved={handleSavedPhoto} />
</div>

<style>
  .list-pane {
    grid-area: list;
    border-right: 1px solid var(--border);
    background: var(--bg-dominant);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .detail-pane {
    grid-area: detail;
    background: var(--bg-dominant);
    overflow: auto;
    padding: var(--sp-md);
  }

  .error {
    padding: var(--sp-md);
    color: var(--destructive);
  }

  .loading {
    padding: var(--sp-md);
    color: var(--text-muted);
  }
</style>
