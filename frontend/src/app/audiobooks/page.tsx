'use client'

// Core
import { useEffect, useRef, useState } from 'react'
import Link from 'next/link'
// Hooks
import { useTranslations } from '@/hooks/useTranslations'
// Services
import { deleteBook, listBooks, uploadBook } from '@/services/audiobooks'
// Types
import type { Book } from '@/types'
// Utils
import { bookStatusLabel } from '@/utils/format'

export default function AudiobookLibraryPage() {
  const { t } = useTranslations()
  const [books, setBooks] = useState<Book[]>([])
  const [loading, setLoading] = useState(true)
  const [uploading, setUploading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const fileRef = useRef<HTMLInputElement>(null)

  async function refresh() {
    try {
      setBooks(await listBooks())
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    refresh()
  }, [])

  async function handleUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    setUploading(true)
    setError(null)
    try {
      await uploadBook(file)
      await refresh()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setUploading(false)
      if (fileRef.current) fileRef.current.value = ''
    }
  }

  async function handleDelete(id: string) {
    if (!window.confirm(t.audiobooks.confirmDelete)) return
    try {
      await deleteBook(id)
      setBooks((prev) => prev.filter((b) => b.id !== id))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  return (
    <div className="max-w-5xl mx-auto px-6 py-8">
      <div className="flex items-start justify-between gap-4 mb-8">
        <div>
          <h1 className="text-2xl font-bold text-zinc-900 dark:text-white">{t.audiobooks.title}</h1>
          <p className="text-sm text-zinc-500 dark:text-zinc-400 mt-1">{t.audiobooks.subtitle}</p>
        </div>
        <div>
          <input
            ref={fileRef}
            type="file"
            accept=".txt,.epub,.fb2"
            onChange={handleUpload}
            className="hidden"
          />
          <button
            onClick={() => fileRef.current?.click()}
            disabled={uploading}
            className="px-4 py-2 bg-zinc-900 dark:bg-white text-white dark:text-zinc-900 rounded-xl text-sm font-medium hover:bg-zinc-700 dark:hover:bg-zinc-200 disabled:opacity-50 transition-colors"
          >
            {uploading ? t.audiobooks.uploading : t.audiobooks.upload}
          </button>
        </div>
      </div>

      {error && (
        <div className="mb-6 px-4 py-3 rounded-xl bg-red-50 dark:bg-red-950/40 text-red-700 dark:text-red-300 text-sm">
          {t.audiobooks.error}: {error}
        </div>
      )}

      {loading ? (
        <p className="text-sm text-zinc-400">…</p>
      ) : books.length === 0 ? (
        <div className="text-center py-20 text-zinc-400 dark:text-zinc-500">
          <div className="text-4xl mb-4">📚</div>
          <p>{t.audiobooks.empty}</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {books.map((book) => (
            <div
              key={book.id}
              className="group relative rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 p-4 hover:border-zinc-300 dark:hover:border-zinc-700 transition-colors"
            >
              <Link href={`/audiobooks/${book.id}`} className="block">
                <h2 className="font-semibold text-zinc-900 dark:text-white truncate">{book.title}</h2>
                <p className="text-xs text-zinc-500 dark:text-zinc-400 mt-0.5 truncate">
                  {book.author || t.audiobooks.unknownAuthor}
                </p>
                <span className="inline-block mt-3 text-[11px] px-2 py-0.5 rounded-full bg-zinc-200 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-300">
                  {bookStatusLabel(book.status)}
                </span>
              </Link>
              <button
                onClick={() => handleDelete(book.id)}
                className="absolute top-3 right-3 opacity-0 group-hover:opacity-100 text-xs text-zinc-400 hover:text-red-500 transition-opacity"
                aria-label={t.audiobooks.delete}
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
