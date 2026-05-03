<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    requestThumbnail,
    formatError,
    type PhotoSummary,
    type FolderFooter,
  } from "../ipc";
  import PhotoRow from "./PhotoRow.svelte";
  import FooterSummary from "./FooterSummary.svelte";

  interface Props {
    items: PhotoSummary[];
    footer: FolderFooter;
    selectedId: string | null;
    thumbReady: Record<string, number>;
    /** WR-06: per-id failure tick from app.svelte. When the tick for an
     *  id changes, drop the id from `requested` and re-observe the row
     *  so the IntersectionObserver retries on next viewport entry. */
    thumbFailed: Record<string, number>;
    loading: boolean;
    onSelect: (id: string) => void;
  }
  let {
    items,
    footer,
    selectedId,
    thumbReady,
    thumbFailed,
    loading,
    onSelect,
  }: Props = $props();

  let listEl: HTMLElement | null = $state(null);
  let observer: IntersectionObserver | null = null;
  let requested: Set<string> = new Set();

  // Fire-and-forget request for a row's thumbnail. Idempotent on the Rust side.
  async function ensureThumb(id: string) {
    if (requested.has(id)) return;
    requested.add(id);
    try {
      await requestThumbnail(id);
    } catch (e) {
      // Logged but not surfaced: row keeps the placeholder per UI-SPEC.
      console.warn("[requestThumbnail]", id, formatError(e));
    }
  }

  onMount(() => {
    observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            const id = (entry.target as HTMLElement).dataset.photoId;
            if (id) ensureThumb(id);
          }
        }
      },
      { root: listEl, rootMargin: "200px", threshold: 0 },
    );
  });

  onDestroy(() => {
    observer?.disconnect();
    observer = null;
  });

  // Re-attach observer when items change (folder reload).
  $effect(() => {
    void items;
    requested.clear();
    if (!observer || !listEl) return;
    observer.disconnect();
    const rows = listEl.querySelectorAll(".row-anchor");
    rows.forEach((r) => observer!.observe(r));
  });

  // WR-06: watch the per-id failure tick. For each id whose tick is
  // non-zero, drop it from `requested` and (if its row is still in the
  // DOM) re-observe so the next viewport entry retries the request.
  // Tracking ticks is per-id so a second failure for the same id also
  // re-triggers retry (Map-of-counters, not Set-of-ids).
  let lastSeenFailTick: Map<string, number> = new Map();
  $effect(() => {
    if (!listEl) return;
    for (const [id, tick] of Object.entries(thumbFailed)) {
      if (lastSeenFailTick.get(id) === tick) continue;
      lastSeenFailTick.set(id, tick);
      requested.delete(id);
      if (!observer) continue;
      const row = listEl.querySelector(
        `.row-anchor[data-photo-id="${CSS.escape(id)}"]`,
      );
      if (row) observer.observe(row);
    }
  });

  // Keyboard nav (UI-SPEC: arrow up/down moves selection, wrapping).
  function handleKeydown(e: KeyboardEvent) {
    if (items.length === 0) return;
    if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
    e.preventDefault();
    const idx = items.findIndex((i) => i.id === selectedId);
    let next: number;
    if (idx === -1) {
      next = 0;
    } else if (e.key === "ArrowDown") {
      next = (idx + 1) % items.length;
    } else {
      next = (idx - 1 + items.length) % items.length;
    }
    onSelect(items[next].id);
  }
</script>

<div
  class="list"
  bind:this={listEl}
  role="listbox"
  tabindex="0"
  aria-label="Photo list"
  onkeydown={handleKeydown}
>
  {#each items as item (item.id)}
    <div class="row-anchor" data-photo-id={item.id}>
      <PhotoRow
        summary={item}
        selected={item.id === selectedId}
        thumbVersion={thumbReady[item.id] ?? 0}
        onclick={() => onSelect(item.id)}
      />
    </div>
  {/each}
</div>

<FooterSummary {footer} {loading} />

<style>
  .list {
    flex: 1;
    overflow-y: auto;
    outline: none;
  }
  .list:focus-visible {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
</style>
