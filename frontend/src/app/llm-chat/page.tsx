'use client'

// Core
import { useState, useRef, useEffect } from 'react'
// Services
import { sendChatMessage } from '@/services/api'
// Store
import { useChatStore } from '@/store/chatStore'

export default function LLMChatPage() {
  const { messages, isLoading, addMessage, setLoading, clearMessages } = useChatStore()
  const [input, setInput] = useState('')
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  async function handleSend() {
    const text = input.trim()
    if (!text || isLoading) return

    setInput('')
    addMessage({ role: 'user', content: text })
    setLoading(true)

    try {
      const reply = await sendChatMessage(text, messages)
      addMessage({ role: 'assistant', content: reply })
    } catch (err) {
      addMessage({
        role: 'assistant',
        content: `Error: ${err instanceof Error ? err.message : 'Something went wrong'}`,
      })
    } finally {
      setLoading(false)
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  return (
    <div className="flex flex-col h-screen">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-zinc-200 dark:border-zinc-800">
        <div>
          <h1 className="text-lg font-semibold text-zinc-900 dark:text-white">LLM Chat</h1>
          <p className="text-xs text-zinc-400 dark:text-zinc-500">Gemini 2.5 Flash</p>
        </div>
        <button
          onClick={clearMessages}
          className="text-xs text-zinc-400 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 px-3 py-1.5 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
        >
          Clear
        </button>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <div className="text-4xl mb-4">✦</div>
            <p className="text-zinc-500 dark:text-zinc-400 text-lg font-medium">Start a conversation</p>
            <p className="text-zinc-400 dark:text-zinc-600 text-sm mt-1">Ask Gemini anything</p>
          </div>
        )}

        {messages.map((msg) => (
          <div
            key={msg.id}
            className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
          >
            <div
              className={`max-w-[75%] px-4 py-3 rounded-2xl text-sm leading-relaxed whitespace-pre-wrap ${
                msg.role === 'user'
                  ? 'bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 rounded-br-sm'
                  : 'bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 rounded-bl-sm'
              }`}
            >
              {msg.content}
            </div>
          </div>
        ))}

        {isLoading && (
          <div className="flex justify-start">
            <div className="bg-zinc-100 dark:bg-zinc-800 px-4 py-3 rounded-2xl rounded-bl-sm">
              <span className="flex gap-1">
                <span className="w-1.5 h-1.5 bg-zinc-400 rounded-full animate-bounce [animation-delay:-0.3s]" />
                <span className="w-1.5 h-1.5 bg-zinc-400 rounded-full animate-bounce [animation-delay:-0.15s]" />
                <span className="w-1.5 h-1.5 bg-zinc-400 rounded-full animate-bounce" />
              </span>
            </div>
          </div>
        )}
        <div ref={bottomRef} />
      </div>

      {/* Input */}
      <div className="px-6 py-4 border-t border-zinc-200 dark:border-zinc-800">
        <div className="flex gap-3 items-end">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a message... (Enter to send)"
            rows={1}
            className="flex-1 bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 placeholder-zinc-400 dark:placeholder-zinc-500 rounded-xl px-4 py-3 text-sm resize-none focus:outline-none focus:ring-1 focus:ring-zinc-300 dark:focus:ring-zinc-600 max-h-40"
            style={{ height: 'auto' }}
            onInput={(e) => {
              const el = e.currentTarget
              el.style.height = 'auto'
              el.style.height = Math.min(el.scrollHeight, 160) + 'px'
            }}
          />
          <button
            onClick={handleSend}
            disabled={isLoading || !input.trim()}
            className="px-4 py-3 bg-zinc-900 dark:bg-white text-white dark:text-zinc-900 rounded-xl text-sm font-medium hover:bg-zinc-700 dark:hover:bg-zinc-200 disabled:opacity-40 disabled:cursor-not-allowed transition-colors shrink-0"
          >
            Send
          </button>
        </div>
      </div>
    </div>
  )
}
