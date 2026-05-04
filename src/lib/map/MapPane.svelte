<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import L from "leaflet";
  import "leaflet/dist/leaflet.css";
  import "./leaflet-icon-fix";

  interface Props {
    initialCenter: { lat: number; lng: number };
    initialZoom: number;
    onPinChange: (lat: number, lng: number) => void;
  }
  let { initialCenter, initialZoom, onPinChange }: Props = $props();

  // CRITICAL: these are plain `let`, NOT $state. Leaflet mutates internal
  // state on every interaction; $state would trigger spurious Svelte
  // reactivity and break the map. (RESEARCH §Pattern 6.)
  let mapEl: HTMLDivElement;
  let map: L.Map | null = null;
  let marker: L.Marker | null = null;

  onMount(() => {
    map = L.map(mapEl, {
      zoomControl: true,
      // We add our own attribution control so we can drop Leaflet's
      // "Leaflet | " prefix and keep only the OSM credit (APP-04).
      attributionControl: false,
    }).setView([initialCenter.lat, initialCenter.lng], initialZoom);

    L.tileLayer("pfp-tile://osm/{z}/{x}/{y}.png", {
      maxZoom: 19,
      // D-35 / Pitfall 5: do NOT prefetch off-viewport tiles.
      keepBuffer: 0,
      attribution: "© OpenStreetMap contributors",
    }).addTo(map);

    L.control
      .attribution({ prefix: false })
      .addAttribution("© OpenStreetMap contributors")
      .addTo(map);

    marker = L.marker([initialCenter.lat, initialCenter.lng], {
      draggable: true,
    }).addTo(map);

    // D-29: drag fine-tunes the pin position. dragend fires once per drag.
    marker.on("dragend", () => {
      if (!marker) return;
      const ll = marker.getLatLng();
      onPinChange(ll.lat, ll.lng);
    });

    // D-29: single click drops/moves the pin. Map does NOT pan to the
    // click location — the pin moves under the user's cursor; the user's
    // mental "this is where I just clicked" is preserved.
    map.on("click", (e: L.LeafletMouseEvent) => {
      if (!marker || !map) return;
      marker.setLatLng(e.latlng);
      onPinChange(e.latlng.lat, e.latlng.lng);
    });
  });

  onDestroy(() => {
    if (map) {
      map.remove();
      map = null;
      marker = null;
    }
  });

  // Reactive: when the parent prop initialCenter changes (selection of
  // a different photo), recenter and move the pin WITHOUT unmounting
  // Leaflet (UI-SPEC §Layout reflow: "Do not unmount/remount Leaflet on
  // every selection change").
  $effect(() => {
    if (!map || !marker) return;
    map.setView([initialCenter.lat, initialCenter.lng], initialZoom);
    marker.setLatLng([initialCenter.lat, initialCenter.lng]);
  });
</script>

<div bind:this={mapEl} class="map-root"></div>

<style>
  .map-root {
    width: 100%;
    height: 100%;
    /* min-height enforced by parent per UI-SPEC §Detail pane internal layout. */
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }
</style>
