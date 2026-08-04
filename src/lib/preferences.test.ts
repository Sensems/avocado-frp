import { beforeEach, describe, expect, it } from "vitest";
import {
  LANGUAGE_STORAGE_KEY,
  LEGACY_LANGUAGE_STORAGE_KEY,
  loadLocale,
  persistLocale,
} from "./preferences";

describe("language preferences", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("defaults to zh and writes the canonical key", () => {
    expect(loadLocale()).toBe("zh");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("zh");
  });

  it("migrates the legacy language key", () => {
    localStorage.setItem(LEGACY_LANGUAGE_STORAGE_KEY, "en");

    expect(loadLocale()).toBe("en");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("en");
    expect(localStorage.getItem(LEGACY_LANGUAGE_STORAGE_KEY)).toBeNull();
  });

  it("prefers a valid canonical value over the legacy value", () => {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, "zh");
    localStorage.setItem(LEGACY_LANGUAGE_STORAGE_KEY, "en");

    expect(loadLocale()).toBe("zh");
  });

  it("normalizes unsupported values before persisting", () => {
    expect(persistLocale("fr")).toBe("zh");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("zh");
  });
});
