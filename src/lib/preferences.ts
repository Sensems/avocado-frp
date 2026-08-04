export const LANGUAGE_STORAGE_KEY = "avocado-frp-lang";
export const LEGACY_LANGUAGE_STORAGE_KEY = "frp-desktop-lang";

export const SUPPORTED_LOCALES = ["zh", "en"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

type StorageLike = Pick<Storage, "getItem" | "setItem" | "removeItem">;

const isSupportedLocale = (value: string | null): value is SupportedLocale =>
  value !== null && SUPPORTED_LOCALES.includes(value as SupportedLocale);

export const loadLocale = (
  storage: StorageLike = localStorage,
): SupportedLocale => {
  const canonical = storage.getItem(LANGUAGE_STORAGE_KEY);
  const legacy = storage.getItem(LEGACY_LANGUAGE_STORAGE_KEY);
  const locale = isSupportedLocale(canonical)
    ? canonical
    : isSupportedLocale(legacy)
      ? legacy
      : "zh";

  storage.setItem(LANGUAGE_STORAGE_KEY, locale);
  if (legacy !== null) {
    storage.removeItem(LEGACY_LANGUAGE_STORAGE_KEY);
  }
  return locale;
};

export const persistLocale = (
  locale: string,
  storage: StorageLike = localStorage,
): SupportedLocale => {
  const normalized = isSupportedLocale(locale) ? locale : "zh";
  storage.setItem(LANGUAGE_STORAGE_KEY, normalized);
  return normalized;
};
