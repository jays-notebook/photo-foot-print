// src/lib/ipc.ts
//
// Typed wrappers for Phase 2's IPC surface. Mirrors the Rust wire DTOs in
// crates/src-tauri/src/commands/{folder,state,thumbnail}.rs and the
// WireError enum in crates/src-tauri/src/error.rs.
//
// snake_case on the wire (matches Rust serde) — do not camelCase these.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ============================================================================
// Wire DTOs (must match Rust exactly, including field names)
// ============================================================================

export interface PhotoSummary {
  id: string;
  file_name: string;
  has_gps: boolean;
  capture_time: string | null;
  size_bytes: number;
  mtime_unix: number;
}

export interface FolderFooter {
  total_jpegs: number;
  non_image_hidden: number;
  read_failed: number;
}

export interface FolderListing {
  folder_path: string;
  items: PhotoSummary[];
  footer: FolderFooter;
  thumb_cache_writable: boolean;
}

export interface GpsCoord {
  lat: number;
  lng: number;
}

export interface PhotoMeta {
  id: string;
  file_name: string;
  gps: GpsCoord | null;
  altitude_m: number | null;
  capture_time: string | null;
  dimensions: [number, number] | null;
}

export type LastFolderStatus =
  | { kind: "none" }
  | { kind: "available"; path: string }
  | { kind: "missing"; path: string };

export interface AppStateDto {
  last_folder: LastFolderStatus;
}

export type RequestThumbnailAck =
  | { status: "ready" }
  | { status: "in_flight" }
  | { status: "queued" };

export type WireError =
  | { kind: "io"; detail: string }
  | { kind: "exif"; detail: string }
  | { kind: "path_traversal"; detail: string }
  | { kind: "photos"; detail: string }
  | { kind: "state"; detail: string }
  | { kind: "exif_write"; detail: string };

export interface ThumbnailReadyPayload {
  id: string;
}

export interface ThumbnailFailedPayload {
  id: string;
  reason: string;
}

// ============================================================================
// Typed invoke wrappers
// ============================================================================

export async function getAppState(): Promise<AppStateDto> {
  return await invoke<AppStateDto>("get_app_state");
}

/** Returns a canonicalized folder path string, or null if the user cancelled. */
export async function openFolderDialog(): Promise<string | null> {
  return await invoke<string | null>("open_folder_dialog");
}

export async function listFolder(path: string): Promise<FolderListing> {
  return await invoke<FolderListing>("list_folder", { path });
}

export async function readPhotoMeta(id: string): Promise<PhotoMeta> {
  return await invoke<PhotoMeta>("read_photo_meta", { id });
}

export async function requestThumbnail(id: string): Promise<RequestThumbnailAck> {
  return await invoke<RequestThumbnailAck>("request_thumbnail", { id });
}

/** Phase 3: write GPS coords to the photo's EXIF. Returns the fresh PhotoMeta
 *  re-read from the file after the atomic write completes. On any failure,
 *  the original file is unchanged (Phase 1 D-07 atomic-write contract); the
 *  error surfaces as WireError with kind === "exif_write". */
export async function saveGeotag(
  id: string,
  lat: number,
  lng: number,
): Promise<PhotoMeta> {
  return await invoke<PhotoMeta>("save_geotag", { id, lat, lng });
}

/** Phase 3: read the in-session last-saved pin (D-27/D-28). Returns null if
 *  no save has happened yet this session. Phase 4 will replace this with a
 *  cross-launch persisted version via pfp-state. */
export async function getSessionLastPin(): Promise<GpsCoord | null> {
  return await invoke<GpsCoord | null>("get_session_last_pin");
}

// ============================================================================
// Event subscriptions
// ============================================================================

export function onThumbnailReady(
  handler: (id: string) => void,
): Promise<UnlistenFn> {
  return listen<ThumbnailReadyPayload>("thumbnail-ready", (e) => {
    handler(e.payload.id);
  });
}

/** WR-06: subscribe to per-id decode failures so the frontend can drop
 *  the id from its in-flight set and let the IntersectionObserver retry
 *  on next viewport entry. The row still keeps the placeholder icon. */
export function onThumbnailFailed(
  handler: (id: string, reason: string) => void,
): Promise<UnlistenFn> {
  return listen<ThumbnailFailedPayload>("thumbnail-failed", (e) => {
    handler(e.payload.id, e.payload.reason);
  });
}

// ============================================================================
// Helpers
// ============================================================================

/** Type guard: best-effort discrimination of WireError vs string vs unknown. */
export function asWireError(e: unknown): WireError | null {
  if (typeof e === "object" && e !== null && "kind" in e && "detail" in e) {
    return e as WireError;
  }
  return null;
}

/** Format a wire-error or fallback string for inline display. */
export function formatError(e: unknown): string {
  const wire = asWireError(e);
  if (wire) return `${wire.kind}: ${wire.detail}`;
  return typeof e === "string" ? e : JSON.stringify(e);
}

/** Truncate a path for the toolbar label using middle-ellipsis (D-12 / UI-SPEC). */
export function truncateMiddle(path: string, max = 48): string {
  if (path.length <= max) return path;
  const keep = Math.floor((max - 3) / 2);
  return `${path.slice(0, keep)}...${path.slice(path.length - keep)}`;
}

/** Decimal lat/lng with exactly 6 decimal places (UI-SPEC detail readout). */
export function formatLatLng(value: number): string {
  return value.toFixed(6);
}

/** Display capture time: convert "YYYY:MM:DD HH:MM:SS" -> "YYYY-MM-DD HH:MM:SS"
 *  (UI-SPEC: dashes for readability; on-disk format unchanged).
 *
 *  WR-07: returns `null` when the input does NOT match the expected
 *  19-char DTO shape, so the caller can render the muted "—" placeholder
 *  instead of a verbatim malformed string. The Rust-side
 *  `validate_dto_format` already filters bad inputs before they reach
 *  the wire, so reaching the null branch implies a contract violation
 *  -- still safer to render a placeholder than to mislead the user with
 *  a raw "not a date" string. */
export function formatCaptureTime(s: string): string | null {
  if (s.length < 19) return null;
  if (s[4] !== ":" || s[7] !== ":" || s[10] !== " ") return null;
  return `${s.slice(0, 4)}-${s.slice(5, 7)}-${s.slice(8, 10)}${s.slice(10)}`;
}
