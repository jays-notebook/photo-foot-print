import { expect, test, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { saveGeotag, type CaptureTimeChange } from "./ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

test.each<CaptureTimeChange>([
  { kind: "keep" }, { kind: "remove" },
  { kind: "set", value: "2001:02:03 04:05:00" },
])("serializes capture-time operations at the IPC boundary: %j", async (captureTime) => {
  await saveGeotag("photo", 37, 127, captureTime);
  expect(invoke).toHaveBeenLastCalledWith("save_geotag", {
    id: "photo", lat: 37, lng: 127, captureTime,
  });
});
