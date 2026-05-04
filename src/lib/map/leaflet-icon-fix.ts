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

// L.Icon.Default overrides _getIconUrl to derive the URL from imagePath +
// name, which ignores the merged iconUrl/iconRetinaUrl/shadowUrl options.
// Removing the prototype override lets the base L.Icon._getIconUrl read the
// merged options, which is what makes the marker render under Vite.
delete (
  L.Icon.Default.prototype as { _getIconUrl?: unknown }
)._getIconUrl;

L.Icon.Default.mergeOptions({
  iconUrl,
  iconRetinaUrl,
  shadowUrl,
});
