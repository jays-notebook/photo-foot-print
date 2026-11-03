import { afterEach, describe, it, expect, vi } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";

import {
  buildSaveDialogBody,
  buildSaveDialogTitle,
  confirmSaveGeotag,
  type SaveDialogProps,
} from "./saveDialog";

afterEach(clearMocks);

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

describe("confirmSaveGeotag — native confirmation boundary", () => {
  const branches: Array<[string, () => SaveDialogProps]> = [
    ["coords-only", basePropsCoordsOnly],
    ["dto-only", basePropsDtoOnly],
    ["both", basePropsBoth],
  ];

  for (const [name, makeProps] of branches) {
    it(`${name}: sends the reviewed values to the raw-result confirmation command`, async () => {
      const nativeCommand = vi.fn(() => false);
      mockIPC(nativeCommand);
      const props = makeProps();
      await confirmSaveGeotag(props);
      expect(nativeCommand).toHaveBeenCalledExactlyOnceWith("confirm_save", {
        title: buildSaveDialogTitle(props),
        message: buildSaveDialogBody(props),
      });
    });
  }

  it.each([false, true])("preserves the native approval result %s without inversion", async (approved) => {
    mockIPC(() => approved);
    const result = await confirmSaveGeotag(basePropsBoth());
    expect(result).toBe(approved);
  });

  it("does not approve when the native confirmation command fails", async () => {
    mockIPC(() => { throw new Error("window unavailable"); });
    await expect(confirmSaveGeotag(basePropsBoth())).rejects.toThrow("window unavailable");
  });
});
