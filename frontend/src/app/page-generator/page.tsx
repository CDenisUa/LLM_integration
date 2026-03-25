'use client'

// Core
import { useState } from 'react'
// Hooks
import { useTranslations } from '@/hooks/useTranslations'
// Services
import { generatePage } from '@/services/api'

export default function PageGeneratorPage() {
  const { t } = useTranslations()
  const [prompt, setPrompt] = useState('')
  const [html, setHtml] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')

  async function handleGenerate() {
    const text = prompt.trim()
    if (!text || isLoading) return

    setIsLoading(true)
    setError('')
    setHtml('')

    try {
      const result = await generatePage(text)
      setHtml(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : t.pageGen.generationFailed)
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="flex flex-col h-screen">
      {/* Header */}
      <div className="px-6 py-4 border-b border-zinc-200 dark:border-zinc-800">
        <h1 className="text-lg font-semibold text-zinc-900 dark:text-white">{t.pageGen.title}</h1>
        <p className="text-xs text-zinc-400 dark:text-zinc-500">{t.pageGen.subtitle}</p>
      </div>

      {/* Prompt input */}
      <div className="px-6 py-4 border-b border-zinc-200 dark:border-zinc-800">
        <div className="flex gap-3">
          <input
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleGenerate()}
            placeholder={t.pageGen.placeholder}
            className="flex-1 bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 placeholder-zinc-400 dark:placeholder-zinc-500 rounded-xl px-4 py-3 text-sm focus:outline-none focus:ring-1 focus:ring-zinc-300 dark:focus:ring-zinc-600"
          />
          <button
            onClick={handleGenerate}
            disabled={isLoading || !prompt.trim()}
            className="px-5 py-3 bg-zinc-900 dark:bg-white text-white dark:text-zinc-900 rounded-xl text-sm font-medium hover:bg-zinc-700 dark:hover:bg-zinc-200 disabled:opacity-40 disabled:cursor-not-allowed transition-colors shrink-0"
          >
            {isLoading ? t.pageGen.generating : t.pageGen.generate}
          </button>
        </div>
      </div>

      {/* Preview */}
      <div className="flex-1 relative">
        {!html && !isLoading && !error && (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <div className="text-4xl mb-4">◈</div>
            <p className="text-zinc-500 dark:text-zinc-400 text-lg font-medium">{t.pageGen.emptyTitle}</p>
            <p className="text-zinc-400 dark:text-zinc-600 text-sm mt-1">{t.pageGen.emptySubtitle}</p>
          </div>
        )}

        {isLoading && (
          <div className="flex flex-col items-center justify-center h-full gap-4">
            <div className="w-8 h-8 border-2 border-zinc-300 dark:border-zinc-600 border-t-zinc-900 dark:border-t-white rounded-full animate-spin" />
            <p className="text-zinc-500 dark:text-zinc-400 text-sm">{t.pageGen.building}</p>
          </div>
        )}

        {error && (
          <div className="m-6 p-4 bg-red-50 dark:bg-red-950/50 border border-red-200 dark:border-red-800 rounded-xl text-red-600 dark:text-red-400 text-sm">
            {error}
          </div>
        )}

        {html && (
          <div className="h-full flex flex-col">
            <div className="flex items-center justify-between px-4 py-2 bg-zinc-50 dark:bg-zinc-900 border-b border-zinc-200 dark:border-zinc-800 text-xs text-zinc-500">
              <span>{t.pageGen.preview}</span>
              <button
                onClick={() => {
                  const blob = new Blob([html], { type: 'text/html' })
                  const url = URL.createObjectURL(blob)
                  const a = document.createElement('a')
                  a.href = url
                  a.download = 'generated-page.html'
                  a.click()
                }}
                className="text-zinc-400 hover:text-zinc-900 dark:hover:text-white transition-colors"
              >
                {t.pageGen.download}
              </button>
            </div>
            <iframe
              srcDoc={html}
              className="flex-1 w-full border-0 bg-white"
              title={t.pageGen.previewFrameTitle}
              sandbox="allow-scripts"
            />
          </div>
        )}
      </div>
    </div>
  )
}
