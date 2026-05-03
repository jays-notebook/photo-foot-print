<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    getAppState,
    listFolder,
    onThumbnailReady,
    type FolderListing,
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

  let unlistenThumbReady: UnlistenFn | null = null;

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

    unlistenThumbReady = await onThumbnailReady((id) => {
      // Increment a reload counter so PhotoRow's <img src> remounts.
      thumbReady = { ...thumbReady, [id]: (thumbReady[id] ?? 0) + 1 };
    });
  });

  onDestroy(() => {
    if (unlistenThumbReady) unlistenThumbReady();
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
      {loading}
      onSelect={(id) => (selectedId = id)}
    />
  {:else}
    <p class="loading">Loading...</p>
  {/if}
</div>

<div class="detail-pane">
  <DetailPane summary={selectedSummary} />
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
