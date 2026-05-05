// src/lib/coords/parsePaste.ts
//
// Phase 4 (D-49): hand-rolled coordinate paste parser. Two formats only;
// anything else is rejected with `{ error: 'parse' }`. Pitfall 12's
// "silently mis-parsing wrong" is the explicit anti-pattern this rejects.
//
// Format A — decimal pair: '35.6586, 139.7454' or '35.6 139.7' (whitespace
//   and signed-prefix tolerant).
// Format B — Google Maps URL: any string containing 'maps.google',
//   'google.com/maps', or 'goo.gl/maps' AND a '@<lat>,<lng>(,zoom)?'
//   capture. Zoom token is discarded.
//
// Range validation (Pitfall 18): -90 ≤ lat ≤ 90, -180 ≤ lng ≤ 180.
// Out-of-range fails loud — do NOT silently wrap or clamp.
//
// Precision (Pitfall 11): forward full-precision f64. The Rust writer
// truncates to 6 decimals at write time; UI does NOT pre-round.

export type ParseResult =
  | { ok: { lat: number; lng: number } }
  | { error: "parse" };

const DECIMAL_PAIR =
  /^\s*(-?\d{1,3}(?:\.\d+)?)\s*[,\s]\s*(-?\d{1,3}(?:\.\d+)?)\s*$/;

const GMAPS_AT =
  /@(-?\d{1,3}(?:\.\d+)?),(-?\d{1,3}(?:\.\d+)?)(?:,(?:[\d.]+z|[^/]+))?/;

function isGoogleMapsUrl(input: string): boolean {
  const lower = input.toLowerCase();
  return (
    lower.includes("maps.google") ||
    lower.includes("google.com/maps") ||
    lower.includes("goo.gl/maps")
  );
}

function inRange(lat: number, lng: number): boolean {
  return (
    Number.isFinite(lat) &&
    Number.isFinite(lng) &&
    Math.abs(lat) <= 90 &&
    Math.abs(lng) <= 180
  );
}

export function parsePasteCoords(input: string): ParseResult {
  // Format A — decimal pair (try first; cheaper).
  const m1 = input.match(DECIMAL_PAIR);
  if (m1) {
    const lat = Number.parseFloat(m1[1]);
    const lng = Number.parseFloat(m1[2]);
    if (inRange(lat, lng)) return { ok: { lat, lng } };
    return { error: "parse" };
  }

  // Format B — Google Maps URL.
  if (isGoogleMapsUrl(input)) {
    const m2 = input.match(GMAPS_AT);
    if (m2) {
      const lat = Number.parseFloat(m2[1]);
      const lng = Number.parseFloat(m2[2]);
      if (inRange(lat, lng)) return { ok: { lat, lng } };
    }
  }

  return { error: "parse" };
}
