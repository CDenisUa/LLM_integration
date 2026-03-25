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
