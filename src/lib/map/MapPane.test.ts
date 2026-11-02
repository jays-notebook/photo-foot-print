import { afterEach, expect, test, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import L from "leaflet";
import MapPane from "./MapPane.svelte";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
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
