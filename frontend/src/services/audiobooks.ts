// Consts
import { API_URL } from '@/consts/api'
// Types
import type { Book, Chapter, GenerationStatus, Progress } from '@/types'

const BASE = `${API_URL}/api`

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) {
    let message = `Request failed (${res.status})`
    try {
      const body = await res.json()
      message = body.error || message
    } catch {
      /* ignore */
    }
    throw new Error(message)
  }
  return res.json() as Promise<T>
}

export async function listBooks(): Promise<Book[]> {
  return json(await fetch(`${BASE}/books`))
}

export async function getBook(id: string): Promise<Book> {
  return json(await fetch(`${BASE}/books/${id}`))
}

export async function uploadBook(file: File, cover?: File): Promise<Book> {
  const form = new FormData()
  form.append('file', file)
  if (cover) form.append('cover', cover)
  return json(await fetch(`${BASE}/books/upload`, { method: 'POST', body: form }))
}

export async function deleteBook(id: string): Promise<void> {
  const res = await fetch(`${BASE}/books/${id}`, { method: 'DELETE' })
  if (!res.ok) throw new Error(`Delete failed (${res.status})`)
}

export async function extractBook(id: string): Promise<{ chapters: number }> {
  return json(await fetch(`${BASE}/books/${id}/extract`, { method: 'POST' }))
}

export async function cleanBook(id: string): Promise<{ chunks: number }> {
  return json(await fetch(`${BASE}/books/${id}/clean`, { method: 'POST' }))
}

export async function generateBook(id: string): Promise<void> {
  await fetch(`${BASE}/books/${id}/generate`, { method: 'POST' })
}

export async function pauseGeneration(id: string): Promise<void> {
  await fetch(`${BASE}/books/${id}/pause-generation`, { method: 'POST' })
}

export async function resumeGeneration(id: string): Promise<void> {
  await fetch(`${BASE}/books/${id}/resume-generation`, { method: 'POST' })
}

export async function retryGeneration(id: string): Promise<void> {
  await fetch(`${BASE}/books/${id}/retry`, { method: 'POST' })
}

export async function getGeneration(id: string): Promise<GenerationStatus> {
  return json(await fetch(`${BASE}/books/${id}/generation`))
}

export async function listChapters(id: string): Promise<Chapter[]> {
  return json(await fetch(`${BASE}/books/${id}/chapters`))
}

export async function getProgress(id: string): Promise<Progress | null> {
  return json(await fetch(`${BASE}/progress/${id}`))
}

export async function saveProgress(
  id: string,
  body: { chapter_id?: string; position_seconds: number; total_listened: number }
): Promise<void> {
  await fetch(`${BASE}/progress/${id}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
}

/** Map a stored audio path (`storage/books/...`) to its served URL. */
export function audioUrl(path: string): string {
  const rel = path.replace(/^storage\/books\//, '')
  return `${BASE}/audio/${rel}`
}
