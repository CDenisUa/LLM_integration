'use client'

// Core
import { useCallback, useEffect, useRef, useState } from 'react'
import Link from 'next/link'
import { useParams } from 'next/navigation'
// Hooks
import { useTranslations } from '@/hooks/useTranslations'
// Services
import {
  audioUrl,
  cleanBook,
  extractBook,
  generateBook,
  getBook,
  getGeneration,
  getProgress,
  listChapters,
  pauseGeneration,
  retryGeneration,
  saveProgress,
} from '@/services/audiobooks'
// Types
import type { Book, Chapter, GenerationStatus, Progress } from '@/types'
// Utils
import { bookStatusLabel } from '@/utils/format'

export default function BookDetailPage() {
  const { t } = useTranslations()
  const params = useParams<{ id: string }>()
  const id = params.id

  const [book, setBook] = useState<Book | null>(null)
  const [chapters, setChapters] = useState<Chapter[]>([])
  const [gen, setGen] = useState<GenerationStatus | null>(null)
  const [progress, setProgress] = useState<Progress | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const restored = useRef<Set<string>>(new Set())

  const refresh = useCallback(async () => {
    try {
      const [b, ch, g, p] = await Promise.all([
        getBook(id),
        listChapters(id),
        getGeneration(id),
        getProgress(id),
      ])
      setBook(b)
      setChapters(ch)
      setGen(g)
      setProgress(p)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [id])

  useEffect(() => {
    refresh()
  }, [refresh])

  // Poll while a generation job is running.
  useEffect(() => {
    if (!gen?.running) return
    const timer = setInterval(refresh, 2000)
    return () => clearInterval(timer)
  }, [gen?.running, refresh])

  async function run(action: () => Promise<unknown>) {
    setBusy(true)
    setError(null)
    try {
      await action()
      await refresh()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(false)
    }
  }

  function onAudioStop(chapterId: string, el: HTMLAudioElement) {
    void saveProgress(id, {
      chapter_id: chapterId,
      position_seconds: el.currentTime,
      total_listened: el.currentTime,
    })
  }

  if (!book) {
    return (
      <div className="max-w-3xl mx-auto px-6 py-8">
        <Link href="/audiobooks" className="text-sm text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200">
          {t.audiobooks.back}
        </Link>
        {error && <p className="mt-4 text-sm text-red-500">{error}</p>}
      </div>
    )
  }

  const summary = gen?.summary
  const percent = summary?.percent ?? 0

  return (
    <div className="max-w-3xl mx-auto px-6 py-8">
      <Link href="/audiobooks" className="text-sm text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200">
        {t.audiobooks.back}
      </Link>

      <div className="mt-4 mb-6">
        <h1 className="text-2xl font-bold text-zinc-900 dark:text-white">{book.title}</h1>
        <p className="text-sm text-zinc-500 dark:text-zinc-400 mt-1">
          {book.author || t.audiobooks.unknownAuthor}
        </p>
        <span className="inline-block mt-3 text-xs px-2.5 py-1 rounded-full bg-zinc-200 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300">
          {bookStatusLabel(book.status)}
        </span>
      </div>

      {error && (
        <div className="mb-6 px-4 py-3 rounded-xl bg-red-50 dark:bg-red-950/40 text-red-700 dark:text-red-300 text-sm">
          {t.audiobooks.error}: {error}
        </div>
      )}

      {/* Actions */}
      <div className="flex flex-wrap gap-2 mb-6">
        <ActionButton label={t.audiobooks.extract} onClick={() => run(() => extractBook(id))} disabled={busy} />
        <ActionButton label={t.audiobooks.clean} onClick={() => run(() => cleanBook(id))} disabled={busy || chapters.length === 0} />
        {gen?.running ? (
          <ActionButton label={t.audiobooks.pause} onClick={() => run(() => pauseGeneration(id))} disabled={busy} />
        ) : (
          <ActionButton
            label={t.audiobooks.generate}
            primary
            onClick={() => run(() => generateBook(id))}
            disabled={busy || !summary || summary.total === 0}
          />
        )}
        {summary && summary.failed > 0 && (
          <ActionButton label={t.audiobooks.retry} onClick={() => run(() => retryGeneration(id))} disabled={busy} />
        )}
      </div>

      {/* Generation progress */}
      {summary && summary.total > 0 && (
        <div className="mb-8">
          <div className="flex items-center justify-between text-xs text-zinc-500 dark:text-zinc-400 mb-1.5">
            <span>{t.audiobooks.progress}</span>
            <span>
              {summary.generated}/{summary.total} {t.audiobooks.chunks} · {percent}%
              {summary.failed > 0 && ` · ${summary.failed} ✕`}
            </span>
          </div>
          <div className="h-2 rounded-full bg-zinc-200 dark:bg-zinc-800 overflow-hidden">
            <div
              className="h-full bg-zinc-900 dark:bg-white transition-[width] duration-500"
              style={{ width: `${percent}%` }}
            />
          </div>
        </div>
      )}

      {/* Chapters */}
      <h2 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300 mb-3">{t.audiobooks.chapters}</h2>
      {chapters.length === 0 ? (
        <p className="text-sm text-zinc-400">{t.audiobooks.noChapters}</p>
      ) : (
        <ul className="space-y-3">
          {chapters.map((chapter, i) => (
            <li
              key={chapter.id}
              className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 p-4"
            >
              <p className="text-sm font-medium text-zinc-800 dark:text-zinc-200">
                {chapter.title || `${t.audiobooks.chapters} ${i + 1}`}
              </p>
              {chapter.audio_path ? (
                <audio
                  controls
                  className="mt-3 w-full"
                  src={audioUrl(chapter.audio_path)}
                  onLoadedMetadata={(e) => {
                    if (
                      progress?.chapter_id === chapter.id &&
                      !restored.current.has(chapter.id) &&
                      progress.position_seconds > 0
                    ) {
                      e.currentTarget.currentTime = progress.position_seconds
                      restored.current.add(chapter.id)
                    }
                  }}
                  onPause={(e) => onAudioStop(chapter.id, e.currentTarget)}
                  onEnded={(e) => onAudioStop(chapter.id, e.currentTarget)}
                />
              ) : (
                <p className="mt-2 text-xs text-zinc-400">{t.audiobooks.noAudioYet}</p>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

function ActionButton({
  label,
  onClick,
  disabled,
  primary,
}: {
  label: string
  onClick: () => void
  disabled?: boolean
  primary?: boolean
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`px-3.5 py-2 rounded-xl text-sm font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed ${
        primary
          ? 'bg-zinc-900 dark:bg-white text-white dark:text-zinc-900 hover:bg-zinc-700 dark:hover:bg-zinc-200'
          : 'border border-zinc-300 dark:border-zinc-700 text-zinc-700 dark:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800'
      }`}
    >
      {label}
    </button>
  )
}
