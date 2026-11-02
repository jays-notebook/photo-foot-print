// Native save confirmation preserves Cancel as the default on macOS and Windows.
// Rust reads raw dialog results so Escape/window close can never become Save.
import { message } from "@tauri-apps/plugin-dialog";
import { toDisplay } from "../datetime/exifDateTime";
import { confirmSave } from "../ipc";

export interface SaveDialogProps {
  fileName: string;
  // Phase 3 carry-over:
  newLat: number;
  newLng: number;
  oldLat: number | null;
  oldLng: number | null;
  // Phase 4 NEW:
  newDto: string | null; // EXIF wire format 'YYYY:MM:DD HH:MM:SS' or null
  oldDto: string | null;
  dirtyCoords: boolean;
  dirtyDto: boolean;
}

/** Build the body string for confirmSaveGeotag. Exported for unit tests
 *  (UI-SPEC §14 saveDialog.body-* rows). UI-SPEC §8.3 formatting rules:
 *  - Two-space indent on value lines.
 *  - `New: ` / `Old: ` labels; missing prior values render `(none)`.
 *  - 6-decimal coordinate precision (Pitfall 11).
 *  - Capture time displayed as 'YYYY-MM-DD HH:MM:SS' via toDisplay.
 *  - One blank line between {file_name} and the first section.
 *  - One blank line between sections when both are present. */
export function buildSaveDialogBody(props: SaveDialogProps): string {
  const lines: string[] = [props.fileName, ""];

  if (props.dirtyCoords) {
    const oldCoords =
      props.oldLat !== null && props.oldLng !== null
        ? `${props.oldLat.toFixed(6)}, ${props.oldLng.toFixed(6)}`
        : "(none)";
    lines.push("Coordinates:");
    lines.push(
      `  New: ${props.newLat.toFixed(6)}, ${props.newLng.toFixed(6)}`,
    );
    lines.push(`  Old: ${oldCoords}`);
  }

  if (props.dirtyDto) {
    if (props.dirtyCoords) lines.push(""); // blank-line separator (UI-SPEC §8.3)
    lines.push("Capture time:");
    lines.push(`  New: ${toDisplay(props.newDto)}`);
    lines.push(`  Old: ${toDisplay(props.oldDto)}`);
  }

  return lines.join("\n");
}

/** Title flips per D-60 / UI-SPEC §8.1. Exported for unit tests. */
export function buildSaveDialogTitle(props: SaveDialogProps): string {
  // dirtyDto wins regardless of dirtyCoords — once capture time is in
  // play, "geotag" undersells the change.
  return props.dirtyDto ? "Save photo metadata?" : "Save geotag?";
}

/** Returns true if the user confirmed Save, false on Cancel/dismiss/Enter. */
export async function confirmSaveGeotag(
  props: SaveDialogProps,
): Promise<boolean> {
  const body = buildSaveDialogBody(props);
  const title = buildSaveDialogTitle(props);

  return await confirmSave(title, body);
}

/** Phase 3 (D-41): show a native error dialog when save_geotag fails.
 *  The atomic-write contract guarantees the original is unchanged on any
 *  failure path; the body text says so explicitly. */
export async function showSaveError(
  fileName: string,
  detail: string,
): Promise<void> {
  await message(
    `Could not save ${fileName}.\n\n${detail}\n\nThe original file is unchanged.`,
    { title: "Save failed", kind: "error" },
  );
}
