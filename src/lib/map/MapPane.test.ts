import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { clearMocks, mockConvertFileSrc } from "@tauri-apps/api/mocks";
import L from "leaflet";
import MapPane from "./MapPane.svelte";

beforeEach(() => mockConvertFileSrc("macos"));
afterEach(() => {
  cleanup();
  clearMocks();
  vi.restoreAllMocks();
});

test.each([
  ["macos", "pfp-tile://localhost/osm/3/1/2.png"],
  ["windows", "http://pfp-tile.localhost/osm/3/1/2.png"],
] as const)("%s tile URLs preserve Leaflet coordinate substitution", (platform, expected) => {
  mockConvertFileSrc(platform);
  const createTileLayer = vi.spyOn(L, "tileLayer");
  render(MapPane, {
    initialCenter: { lat: 37, lng: 127 }, initialZoom: 3, onPinChange: vi.fn(),
  });
  const tiles = createTileLayer.mock.results[0].value as L.TileLayer;
  expect(tiles.getTileUrl(Object.assign(L.point(1, 2), { z: 3 }))).toBe(expected);
});

test.each([
  [181, -179],
  [-181, 179],
  [541, -179],
])("map click wraps longitude %s to %s before editing GPS", (lng, expected) => {
  const createMap = vi.spyOn(L, "map");
  const onPinChange = vi.fn();
  render(MapPane, {
    initialCenter: { lat: 37, lng: 127 }, initialZoom: 2, onPinChange,
  });
  const map = createMap.mock.results[0].value as L.Map;

  map.fire("click", { latlng: L.latLng(37, lng) });

  expect(onPinChange).toHaveBeenLastCalledWith(37, expected);
});

test.each([
  [181, -179],
  [-181, 179],
])("marker drag wraps longitude %s to %s before editing GPS", (lng, expected) => {
  const createMarker = vi.spyOn(L, "marker");
  const onPinChange = vi.fn();
  render(MapPane, {
    initialCenter: { lat: 37, lng: 127 }, initialZoom: 2, onPinChange,
  });
  const marker = createMarker.mock.results[0].value as L.Marker;

  marker.setLatLng([37, lng]).fire("dragend");

  expect(onPinChange).toHaveBeenLastCalledWith(37, expected);
});
