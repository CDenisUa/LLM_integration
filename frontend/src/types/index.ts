export interface Message {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: Date
}

export interface NavItem {
  label: string
  href?: string
  children?: NavItem[]
}

export interface Book {
  id: string
  title: string
  author: string | null
  original_path: string | null
  cover_path: string | null
  status: string
  created_at: number
  updated_at: number
}

export interface Chapter {
  id: string
  book_id: string
  title: string | null
  order_index: number
  text: string
  status: string
  audio_path: string | null
  duration: number | null
}

export interface GenerationSummary {
  total: number
  generated: number
  failed: number
  pending: number
  percent: number
}

export interface GenerationStatus {
  summary: GenerationSummary
  status: string | null
  running: boolean
}

export interface Progress {
  book_id: string
  chapter_id: string | null
  position_seconds: number
  total_listened: number
  last_opened_at: number | null
}
