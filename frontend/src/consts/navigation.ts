// Types
import type { NavItem } from '@/types'

export const NAV_ITEMS: NavItem[] = [
  {
    label: 'LLM Chat',
    children: [
      { label: 'Gemini', href: '/llm-chat' },
    ],
  },
  {
    label: 'Page Generation',
    children: [
      { label: 'Generator', href: '/page-generator' },
    ],
  },
]
