import type { Locale } from "@ai-usage-dashboard/core"
import { en } from "./en"
import { ko } from "./ko"
import type { TranslationKey } from "./keys"

const bundles: Record<Locale, Record<TranslationKey, string>> = { ko, en }

export type TranslationParams = Record<string, string | number>
export type TFunction = (key: TranslationKey, params?: TranslationParams) => string

export function translate(locale: Locale, key: TranslationKey, params?: TranslationParams): string {
  const template = bundles[locale][key] ?? bundles.ko[key] ?? key
  if (!params) return template
  return template.replace(/\{(\w+)\}/g, (_, name: string) => {
    const value = params[name]
    return value === undefined ? `{${name}}` : String(value)
  })
}

export function createT(locale: Locale): TFunction {
  return (key, params) => translate(locale, key, params)
}

export type { Locale, TranslationKey }
