// src/lib/coords/parsePaste.test.ts
//
// Phase 4 (UI-SPEC §14): unit tests for the hand-rolled paste parser.
// Locks D-49 + Pitfall 12 + Pitfall 18.

import { describe, it, expect } from "vitest";
import { parsePasteCoords } from "./parsePaste";

describe("parsePasteCoords", () => {
  it("decimal-ok: '35.6586, 139.7454'", () => {
    expect(parsePasteCoords("35.6586, 139.7454")).toEqual({
      ok: { lat: 35.6586, lng: 139.7454 },
    });
  });

  it("decimal-tabbed: tolerates whitespace mix", () => {
    expect(parsePasteCoords("  35.6 \t 139.7 ")).toEqual({
      ok: { lat: 35.6, lng: 139.7 },
    });
  });

  it("decimal-signed: south/east hemisphere round-trips", () => {
    expect(parsePasteCoords("-33.87, 151.21")).toEqual({
      ok: { lat: -33.87, lng: 151.21 },
    });
  });

  it("decimal-space-separator: space alone qualifies as separator", () => {
    expect(parsePasteCoords("35.6586 139.7454")).toEqual({
      ok: { lat: 35.6586, lng: 139.7454 },
    });
  });

  it("google-url-ok: discards zoom token", () => {
    expect(
      parsePasteCoords("https://www.google.com/maps/@35.6586,139.7454,15z"),
    ).toEqual({ ok: { lat: 35.6586, lng: 139.7454 } });
  });

  it("google-url-no-zoom: optional zoom group", () => {
    expect(
      parsePasteCoords("https://www.google.com/maps/@35.6586,139.7454"),
    ).toEqual({ ok: { lat: 35.6586, lng: 139.7454 } });
  });

  it("google-short-url: goo.gl/maps host accepted", () => {
    expect(
      parsePasteCoords("https://goo.gl/maps/foo?@35.6586,139.7454,17z"),
    ).toEqual({ ok: { lat: 35.6586, lng: 139.7454 } });
  });

  it("range-rejection-lat: '91, 0' fails loud", () => {
    expect(parsePasteCoords("91, 0")).toEqual({ error: "parse" });
  });

  it("range-rejection-lng: '0, 181' fails loud", () => {
    expect(parsePasteCoords("0, 181")).toEqual({ error: "parse" });
  });

  it("range-rejection-negative: '-91, 0' and '0, -181' both fail", () => {
    expect(parsePasteCoords("-91, 0")).toEqual({ error: "parse" });
    expect(parsePasteCoords("0, -181")).toEqual({ error: "parse" });
  });

  it("junk-rejection: 'hello world' fails", () => {
    expect(parsePasteCoords("hello world")).toEqual({ error: "parse" });
  });

  it("empty-rejection: '' and '   ' fail", () => {
    expect(parsePasteCoords("")).toEqual({ error: "parse" });
    expect(parsePasteCoords("   ")).toEqual({ error: "parse" });
  });

  it("dms-rejected: D-49 explicit two-format scope", () => {
    expect(
      parsePasteCoords("35°39'30.96\"N, 139°44'43.44\"E"),
    ).toEqual({ error: "parse" });
  });
});
