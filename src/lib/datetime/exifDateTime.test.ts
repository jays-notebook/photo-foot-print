// src/lib/datetime/exifDateTime.test.ts
//
// Phase 4 (UI-SPEC §14): unit tests for HTML↔EXIF format conversion.

import { describe, it, expect } from "vitest";
import { toHtml, toExif, toDisplay } from "./exifDateTime";

describe("toHtml", () => {
  it("exif-to-html: drops seconds", () => {
    expect(toHtml("2025:08:14 13:42:31")).toBe("2025-08-14T13:42");
  });

  it("null-to-empty-html: null collapses to ''", () => {
    expect(toHtml(null)).toBe("");
  });

  it("toHtml-malformed: invalid input collapses to ''", () => {
    expect(toHtml("not-a-date")).toBe("");
  });
});

describe("toExif", () => {
  it("html-to-exif: appends :00 seconds", () => {
    expect(toExif("2025-08-14T13:42")).toBe("2025:08:14 13:42:00");
  });

  it("empty-html-to-null: '' → null", () => {
    expect(toExif("")).toBeNull();
  });

  it("partial-html-to-null: '2025-08' → null", () => {
    expect(toExif("2025-08")).toBeNull();
  });
});

describe("toDisplay", () => {
  it("toDisplay-canonical: dashes in date, seconds preserved", () => {
    expect(toDisplay("2025:08:14 13:42:31")).toBe("2025-08-14 13:42:31");
  });

  it("toDisplay-null: returns '(none)'", () => {
    expect(toDisplay(null)).toBe("(none)");
  });
});
