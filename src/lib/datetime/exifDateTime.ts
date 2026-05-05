// src/lib/datetime/exifDateTime.ts
//
// Phase 4 (D-56 / D-61): format conversion between the EXIF
// 'YYYY:MM:DD HH:MM:SS' 19-char wire format and the HTML
// <input type="datetime-local"> 'YYYY-MM-DDTHH:MM' 16-char value
// format. Plus toDisplay() for the saveDialog body (UI-SPEC §8.3 —
// dashes for readability; seconds preserved).
//
// Validation authority is Rust (pfp_exif::validate_dto_format). These
// helpers are FAST FEEDBACK ONLY — invalid / partial input collapses
// to null (or empty) so the UI never feeds a malformed string into
// pendingDto. Failures past this layer surface as WireError::ExifWrite.

/** EXIF wire ('YYYY:MM:DD HH:MM:SS' or null) → HTML datetime-local
 *  value ('YYYY-MM-DDTHH:MM' or ''). Mirrors formatCaptureTime in
 *  ipc.ts (byte-slice convention). Returns '' for null and for any
 *  input that does not match the 19-char EXIF shape. */
export function toHtml(exif: string | null): string {
  if (exif === null) return "";
  if (exif.length < 19) return "";
  if (exif[4] !== ":" || exif[7] !== ":" || exif[10] !== " ") return "";
  // 'YYYY:MM:DD HH:MM:SS' → 'YYYY-MM-DDTHH:MM'
  return `${exif.slice(0, 4)}-${exif.slice(5, 7)}-${exif.slice(8, 10)}T${exif.slice(11, 16)}`;
}

/** HTML datetime-local value ('YYYY-MM-DDTHH:MM') → EXIF wire
 *  ('YYYY:MM:DD HH:MM:SS') or null. Empty / partial / malformed input
 *  emits null (UI-SPEC §7.3 — browser allows mid-edit; we hold pendingDto
 *  at null until the 16-char shape lands). */
export function toExif(html: string): string | null {
  if (html.length < 16) return null;
  if (html[4] !== "-" || html[7] !== "-" || html[10] !== "T") return null;
  if (html[13] !== ":") return null;
  // 'YYYY-MM-DDTHH:MM' → 'YYYY:MM:DD HH:MM:SS'
  return `${html.slice(0, 4)}:${html.slice(5, 7)}:${html.slice(8, 10)} ${html.slice(11, 16)}:00`;
}

/** EXIF → display ('YYYY-MM-DD HH:MM:SS'). Used by the saveDialog body
 *  builder (UI-SPEC §8.3). Returns '(none)' for null inputs so the
 *  dialog can interpolate the helper directly into 'Old: <toDisplay>'. */
export function toDisplay(exif: string | null): string {
  if (exif === null) return "(none)";
  if (exif.length < 19) return "(none)";
  if (exif[4] !== ":" || exif[7] !== ":" || exif[10] !== " ") return "(none)";
  // 'YYYY:MM:DD HH:MM:SS' → 'YYYY-MM-DD HH:MM:SS'
  return `${exif.slice(0, 4)}-${exif.slice(5, 7)}-${exif.slice(8, 10)}${exif.slice(10)}`;
}
