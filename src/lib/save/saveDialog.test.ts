// src/lib/save/saveDialog.test.ts
//
// Phase 4 (UI-SPEC §14): unit tests for the widened saveDialog body builder
// + title flip + inverted-semantics lock. Pitfall 9 regression guard.
//
// We test buildSaveDialogBody / buildSaveDialogTitle (exported pure helpers)
// and assert that confirmSaveGeotag's ask() call shape preserves the
// inverted-semantics labels on every body branch via a mock of
// @tauri-apps/plugin-dialog.

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock @tauri-apps/plugin-dialog BEFORE importing saveDialog so the
// confirmSaveGeotag tests can capture the ask() options.
const askMock = vi.fn();
const messageMock = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  ask: (...args: unknown[]) => askMock(...args),
  message: (...args: unknown[]) => messageMock(...args),
}));

import {
  buildSaveDialogBody,
  buildSaveDialogTitle,
  confirmSaveGeotag,
  type SaveDialogProps,
} from "./saveDialog";

function basePropsCoordsOnly(): SaveDialogProps {
  return {
    fileName: "DSC_0042.JPG",
    newLat: 35.6586,
    newLng: 139.7454,
    oldLat: null,
    oldLng: null,
    newDto: null,
    oldDto: null,
    dirtyCoords: true,
    dirtyDto: false,
  };
}

function basePropsDtoOnly(): SaveDialogProps {
  return {
    fileName: "DSC_0042.JPG",
    newLat: 0,
    newLng: 0,
    oldLat: null,
    oldLng: null,
    newDto: "2025:08:14 13:42:00",
    oldDto: null,
    dirtyCoords: false,
    dirtyDto: true,
  };
}

function basePropsBoth(): SaveDialogProps {
  return {
    fileName: "DSC_0042.JPG",
    newLat: 35.6586,
    newLng: 139.7454,
    oldLat: 35.658,
    oldLng: 139.745,
    newDto: "2025:08:14 13:42:00",
    oldDto: "2025:08:14 13:00:00",
    dirtyCoords: true,
    dirtyDto: true,
  };
}

describe("buildSaveDialogTitle", () => {
  it("title-coords-only: dirtyCoords && !dirtyDto → 'Save geotag?'", () => {
    expect(buildSaveDialogTitle(basePropsCoordsOnly())).toBe("Save geotag?");
  });

  it("title-dto-involved (dto-only) → 'Save photo metadata?'", () => {
    expect(buildSaveDialogTitle(basePropsDtoOnly())).toBe(
      "Save photo metadata?",
    );
  });

  it("title-dto-involved (both) → 'Save photo metadata?'", () => {
    expect(buildSaveDialogTitle(basePropsBoth())).toBe(
      "Save photo metadata?",
    );
  });
});

describe("buildSaveDialogBody", () => {
  it("body-coords-only: contains Coordinates: block, no Capture time:", () => {
    const body = buildSaveDialogBody(basePropsCoordsOnly());
    expect(body).toContain("DSC_0042.JPG");
    expect(body).toContain("Coordinates:");
    expect(body).toContain("  New: 35.658600, 139.745400");
    expect(body).toContain("  Old: (none)");
    expect(body).not.toContain("Capture time:");
  });

  it("body-dto-only: contains Capture time: block, no Coordinates:", () => {
    const body = buildSaveDialogBody(basePropsDtoOnly());
    expect(body).toContain("DSC_0042.JPG");
    expect(body).toContain("Capture time:");
    expect(body).toContain("  New: 2025-08-14 13:42:00");
    expect(body).toContain("  Old: (none)");
    expect(body).not.toContain("Coordinates:");
  });

  it("body-both: contains both blocks separated by exactly one blank line", () => {
    const body = buildSaveDialogBody(basePropsBoth());
    expect(body).toContain("Coordinates:");
    expect(body).toContain("Capture time:");
    expect(body).toMatch(/Old: 35\.658000, 139\.745000\n\nCapture time:/);
  });

  it("body-old-none-coords: oldLat/oldLng null → 'Old: (none)'", () => {
    const body = buildSaveDialogBody(basePropsCoordsOnly());
    expect(body).toContain("  Old: (none)");
  });

  it("body-old-none-dto: oldDto null → 'Old: (none)'", () => {
    const body = buildSaveDialogBody(basePropsDtoOnly());
    expect(body).toContain("  Old: (none)");
  });
});

describe("confirmSaveGeotag — inverted-semantics lock (Pitfall 9)", () => {
  beforeEach(() => {
    askMock.mockReset();
  });

  const branches: Array<[string, () => SaveDialogProps]> = [
    ["coords-only", basePropsCoordsOnly],
    ["dto-only", basePropsDtoOnly],
    ["both", basePropsBoth],
  ];

  for (const [name, makeProps] of branches) {
    it(`${name}: ask() called with okLabel='Cancel', cancelLabel='Save', kind='warning'`, async () => {
      askMock.mockResolvedValue(true); // user clicked default ("Cancel")
      await confirmSaveGeotag(makeProps());
      expect(askMock).toHaveBeenCalledTimes(1);
      const [, options] = askMock.mock.calls[0];
      expect(options).toMatchObject({
        kind: "warning",
        okLabel: "Cancel",
        cancelLabel: "Save",
      });
    });
  }

  it("confirm-flips-bool: ask()→true means user cancelled (returns false)", async () => {
    askMock.mockResolvedValue(true);
    const result = await confirmSaveGeotag(basePropsBoth());
    expect(result).toBe(false);
  });

  it("confirm-flips-bool: ask()→false means user confirmed (returns true)", async () => {
    askMock.mockResolvedValue(false);
    const result = await confirmSaveGeotag(basePropsBoth());
    expect(result).toBe(true);
  });
});
