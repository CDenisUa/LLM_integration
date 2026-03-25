// Consts
import { API_URL } from '@/consts/api'
// Types
import type { Message } from '@/types'

export async function sendChatMessage(
  message: string,
  history: Message[],
  model = 'gemini-2.5-flash'
): Promise<string> {
  const response = await fetch(`${API_URL}/api/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      message,
      history: history.map((m) => ({ role: m.role, content: m.content })),
      model,
    }),
  })

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || 'Failed to send message')
  }

  const data = await response.json()
  return data.reply
}

export async function generatePage(prompt: string): Promise<string> {
  const response = await fetch(`${API_URL}/api/generate-page`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ prompt }),
  })

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || 'Failed to generate page')
  }

  const data = await response.json()
  return data.html
}
