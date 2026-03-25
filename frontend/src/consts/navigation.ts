// Consts
import type { Translations } from '@/consts/translations'
// Types
import type { NavItem } from '@/types'

export function getNavItems(t: Translations): NavItem[] {
  return [
    {
      label: t.nav.llmChat,
      children: [
        { label: t.nav.gemini, href: '/llm-chat' },
      ],
    },
    {
      label: t.nav.pageGeneration,
      children: [
        { label: t.nav.promptToHtml, href: '/page-generator' },
        { label: t.nav.multiStepPipeline, href: '/advanced-pipeline' },
      ],
    },
  ]
}
