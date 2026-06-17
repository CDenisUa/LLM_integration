// Utils

/** Format a duration in seconds as `H:MM:SS` or `M:SS`. */
export function formatDuration(seconds: number | null | undefined): string {
  if (!seconds || seconds < 0 || !Number.isFinite(seconds)) return '0:00'
  const total = Math.floor(seconds)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  const pad = (n: number) => n.toString().padStart(2, '0')
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`
}

/** Human label for a book status string. */
export function bookStatusLabel(status: string): string {
  const map: Record<string, string> = {
    uploaded: 'Uploaded',
    text_extracted: 'Text extracted',
    ready_for_generation: 'Ready to generate',
    generating_audio: 'Generating…',
    generated: 'Generated',
    failed: 'Failed',
  }
  return map[status] ?? status
}
