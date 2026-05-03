<script lang="ts">
  import { formatLatLng, formatCaptureTime, type PhotoMeta } from "../ipc";

  interface Props {
    meta: PhotoMeta;
  }
  let { meta }: Props = $props();

  let latText = $derived(meta.gps ? formatLatLng(meta.gps.lat) : "—");
  let lngText = $derived(meta.gps ? formatLatLng(meta.gps.lng) : "—");
  let altText = $derived(
    meta.altitude_m !== null ? `${meta.altitude_m.toFixed(1)} m` : null,
  );
  let captureText = $derived(
    meta.capture_time ? formatCaptureTime(meta.capture_time) : "—",
  );
</script>

<dl class="readout">
  <div class="row">
    <dt>Latitude</dt>
    <dd class:muted={!meta.gps}>{latText}</dd>
  </div>
  <div class="row">
    <dt>Longitude</dt>
    <dd class:muted={!meta.gps}>{lngText}</dd>
  </div>
  {#if altText}
    <div class="row">
      <dt>Altitude</dt>
      <dd>{altText}</dd>
    </div>
  {/if}
  <div class="row">
    <dt>Captured</dt>
    <dd class:muted={!meta.capture_time}>{captureText}</dd>
  </div>
</dl>

<style>
  .readout {
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }
  .row {
    display: grid;
    grid-template-columns: 100px 1fr;
    gap: var(--sp-sm);
    border-bottom: 1px solid var(--border);
    padding-bottom: var(--sp-sm);
  }
  dt {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    letter-spacing: 0.5px;
  }
  dd {
    margin: 0;
    font-size: 14px;
    color: var(--text-default);
    font-variant-numeric: tabular-nums;
  }
  dd.muted {
    color: var(--text-muted);
  }
</style>
