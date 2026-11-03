import { afterEach, expect, test, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { clearMocks, mockConvertFileSrc } from "@tauri-apps/api/mocks";
import PhotoRow from "./PhotoRow.svelte";

afterEach(() => {
  cleanup();
  clearMocks();
});

test.each([
  ["macos", "pfp-thumb://localhost/0123456789abcdef0123456789abcdef"],
  ["windows", "http://pfp-thumb.localhost/0123456789abcdef0123456789abcdef"],
] as const)("%s thumbnail URLs use the native protocol origin and preserve reload versions", async (platform, expected) => {
  mockConvertFileSrc(platform);
  const props = {
    summary: {
      id: "0123456789abcdef0123456789abcdef", file_name: "Scan.jpg",
      has_gps: false, capture_time: null, size_bytes: 100, mtime_unix: 0,
    },
    selected: false, thumbVersion: 0, onclick: vi.fn(),
  };
  const view = render(PhotoRow, props);
  const image = view.container.querySelector("img")!;
  expect(image.getAttribute("src")).toBe(expected);

  await view.rerender({ ...props, thumbVersion: 3 });
  expect(image.getAttribute("src")).toBe(`${expected}?v=3`);
});
