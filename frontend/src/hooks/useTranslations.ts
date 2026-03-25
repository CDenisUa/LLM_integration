'use client'

// Consts
import { translations, type Translations } from '@/consts/translations'
// Store
import { useLocaleStore } from '@/store/localeStore'

export function useTranslations(): { t: Translations; locale: 'en' | 'ru' } {
  const locale = useLocaleStore((state) => state.locale)

  return {
    locale,
    t: translations[locale],
  }
}
