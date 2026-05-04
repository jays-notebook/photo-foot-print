// Leaflet's CSS uses relative URLs to resolve marker icons, which Vite
// does not transform at build time when imported from node_modules.
// The blessed fix: import the bundled icons via Vite's `?url` suffix
// and override L.Icon.Default's options. Without this shim, Leaflet
// markers render as broken-image icons in the production Vite bundle
// (Phase 3 RESEARCH §Pitfall 2; cited https://github.com/Leaflet/Leaflet/issues/7424).
//
// This file is imported for its side effect once, at the top of
// MapPane.svelte. It MUST run before any L.marker(...) call.

import L from "leaflet";
import iconUrl from "leaflet/dist/images/marker-icon.png?url";
import iconRetinaUrl from "leaflet/dist/images/marker-icon-2x.png?url";
import shadowUrl from "leaflet/dist/images/marker-shadow.png?url";

L.Icon.Default.mergeOptions({
  iconUrl,
  iconRetinaUrl,
  shadowUrl,
});
