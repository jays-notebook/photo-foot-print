// Phase 3 native dialog wrappers. D-38 / D-39 / D-41.
//
// confirmSaveGeotag uses the inverted-semantics fallback because
// @tauri-apps/plugin-dialog::ask() does NOT expose a defaultButton parameter
// (RESEARCH VERIFIED FINDING). We pass okLabel='Cancel', cancelLabel='Save',
// then invert the boolean. The macOS native warning dialog renders the OK
// button (now labeled 'Cancel') as the default-focused / rightmost button,
// satisfying D-39's "Enter dismisses without saving" contract.
//
// VERIFICATION GATE: planner flagged this requires a manual smoke check
// on the first dev build (Plan 05). If macOS surfaces the buttons in a
// different order, fall back to a Rust-side MessageDialogBuilder
// .default_button(MessageDialogButton::Cancel) IPC.

import { ask, message } from "@tauri-apps/plugin-dialog";

export interface SaveDialogProps {
  fileName: string;
  newLat: number;
  newLng: number;
  oldLat: number | null;
  oldLng: number | null;
}

/** Returns true if the user confirmed Save, false on Cancel/dismiss/Enter. */
export async function confirmSaveGeotag(
  props: SaveDialogProps,
): Promise<boolean> {
  const oldLine =
    props.oldLat !== null && props.oldLng !== null
      ? `Old:  ${props.oldLat.toFixed(6)}, ${props.oldLng.toFixed(6)}`
      : `Old:  (none)`;

  const body = [
    props.fileName,
    "",
    `New:  ${props.newLat.toFixed(6)}, ${props.newLng.toFixed(6)}`,
    oldLine,
  ].join("\n");

  // Inverted semantics: okLabel='Cancel' → Cancel is the default-focused button.
  // Returned `true` means user clicked the default ("Cancel"); we flip.
  const okIsCancel = await ask(body, {
    title: "Save geotag?",
    kind: "warning",
    okLabel: "Cancel",
    cancelLabel: "Save",
  });
  return !okIsCancel;
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
