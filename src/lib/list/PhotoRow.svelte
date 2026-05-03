<script lang="ts">
  import type { PhotoSummary } from "../ipc";
  import GpsBadge from "./GpsBadge.svelte";

  interface Props {
    summary: PhotoSummary;
    selected: boolean;
    /** Reload counter from app.svelte; non-zero => use src URL with cachebust query. */
    thumbVersion: number;
    onclick: () => void;
  }
  let { summary, selected, thumbVersion, onclick }: Props = $props();

  // The src URL changes whenever thumbVersion changes (forces <img> reload).
  let imgSrc = $derived(
    thumbVersion > 0
      ? `pfp-thumb://${summary.id}?v=${thumbVersion}`
      : `pfp-thumb://${summary.id}`,
  );

  let captureLabel = $derived(summary.capture_time ?? "unknown");
  let gpsLabel = $derived(summary.has_gps ? "has GPS" : "missing GPS");
  let ariaName = $derived(
    `${summary.file_name}, ${gpsLabel}, captured ${captureLabel}`,
  );
</script>

<div
  class="row"
  class:selected
  role="option"
  aria-selected={selected}
  aria-label={ariaName}
  tabindex="-1"
  {onclick}
  onkeydown={(e) => (e.key === "Enter" ? onclick() : null)}
>
  <div class="thumb">
    <svg
      class="placeholder"
      width="32"
      height="32"
      viewBox="0 0 32 32"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      aria-hidden="true"
    >
      <rect x="4" y="6" width="24" height="20" rx="2" />
      <path d="M4 22l8-8 6 6 4-4 6 6" />
    </svg>
    <img
      src={imgSrc}
      alt=""
      width="48"
      height="48"
      loading="lazy"
      onerror={(e) => {
        const t = e.currentTarget as HTMLImageElement;
        t.style.visibility = "hidden";
      }}
      onload={(e) => {
        const t = e.currentTarget as HTMLImageElement;
        t.style.visibility = "visible";
      }}
    />
  </div>
  <div class="meta">
    <div class="name" title={summary.file_name}>{summary.file_name}</div>
    <GpsBadge hasGps={summary.has_gps} />
  </div>
</div>

<style>
  .row {
    height: var(--row-h);
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: var(--sp-sm);
    border-bottom: 1px solid var(--border);
    cursor: default;
    border-left: 3px solid transparent;
    content-visibility: auto;
    contain-intrinsic-size: var(--row-h);
  }
  .row:hover {
    background: var(--bg-secondary);
  }
  .row.selected {
    background: var(--accent-tint-bg);
    border-left-color: var(--accent);
  }

  .thumb {
    position: relative;
    width: 48px;
    height: 48px;
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-radius: 3px;
    overflow: hidden;
  }
  .thumb img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    visibility: hidden;
  }
  .thumb .placeholder {
    position: absolute;
    inset: 0;
    margin: auto;
    color: var(--text-muted);
  }

  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }
  .name {
    font-size: 14px;
    color: var(--text-default);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
