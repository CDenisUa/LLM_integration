'use client'

// Core
import Link from 'next/link'
// Hooks
import { useTranslations } from '@/hooks/useTranslations'

export default function Home() {
  const { t } = useTranslations()

  const sections = [
    {
      title: t.nav.llmChat,
      description: t.home.llmChatDesc,
      items: [{ label: t.nav.gemini, href: '/llm-chat', badge: 'Google AI' }],
    },
    {
      title: t.nav.pageGeneration,
      description: t.home.pageGenDesc,
      items: [
        { label: t.nav.promptToHtml, href: '/page-generator', badge: 'Gemini' },
        { label: t.nav.multiStepPipeline, href: '/advanced-pipeline', badge: t.home.workflowBadge },
      ],
    },
  ]

  return (
    <div className="p-10 max-w-4xl">
      <div className="mb-10">
        <h1 className="text-4xl font-bold text-zinc-900 dark:text-white mb-3">LLM Lab</h1>
        <p className="text-zinc-500 dark:text-zinc-400 text-lg">{t.home.subtitle}</p>
      </div>

      <div className="space-y-8">
        {sections.map((section) => (
          <div key={section.title}>
            <h2 className="text-xl font-semibold text-zinc-800 dark:text-zinc-200 mb-1">{section.title}</h2>
            <p className="text-sm text-zinc-500 mb-4">{section.description}</p>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              {section.items.map((item) => (
                <Link
                  key={item.href}
                  href={item.href}
                  className="group block p-5 rounded-xl bg-zinc-50 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 hover:border-zinc-400 dark:hover:border-zinc-600 transition-colors"
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="font-medium text-zinc-800 dark:text-zinc-100 group-hover:text-zinc-900 dark:group-hover:text-white">
                      {item.label}
                    </span>
                    <span className="text-xs bg-zinc-100 dark:bg-zinc-800 text-zinc-500 dark:text-zinc-400 px-2 py-0.5 rounded-full">
                      {item.badge}
                    </span>
                  </div>
                  <span className="text-zinc-400 dark:text-zinc-600 group-hover:text-zinc-500 text-sm">
                    {t.home.open}
                  </span>
                </Link>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
