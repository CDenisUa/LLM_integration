'use client'

// Core
import { useState, useEffect } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useTheme } from 'next-themes'

// Consts
import { NAV_ITEMS } from '@/consts/navigation'

// Types
import type { NavItem } from '@/types'

function SunIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
    </svg>
  )
}

function MoonIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
    </svg>
  )
}

function ThemeToggle() {
  const { theme, setTheme } = useTheme()
  const [mounted, setMounted] = useState(false)

  useEffect(() => setMounted(true), [])

  if (!mounted) return <div className="w-7 h-7" />

  return (
    <button
      onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
      className="w-7 h-7 flex items-center justify-center rounded-md text-zinc-500 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
      title={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
    >
      {theme === 'dark' ? <SunIcon /> : <MoonIcon />}
    </button>
  )
}

function NavNode({ item, depth = 0 }: { item: NavItem; depth?: number }) {
  const pathname = usePathname()
  const [open, setOpen] = useState(false)
  const hasChildren = item.children && item.children.length > 0
  const isActive = item.href === pathname

  if (hasChildren) {
    return (
      <div>
        <button
          onClick={() => setOpen(!open)}
          className="flex items-center gap-1 w-full px-3 py-1.5 text-sm font-semibold text-zinc-500 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-200 transition-colors"
          style={{ paddingLeft: `${depth * 12 + 12}px` }}
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
            strokeLinecap="round"
            strokeLinejoin="round"
            className={`transition-transform duration-200 shrink-0 ${open ? 'rotate-90' : ''}`}
          >
            <polyline points="3,2 8,6 3,10" />
          </svg>
          {item.label}
        </button>
        {open && (
          <div>
            {item.children!.map((child) => (
              <NavNode key={child.label} item={child} depth={depth + 1} />
            ))}
          </div>
        )}
      </div>
    )
  }

  return (
    <Link
      href={item.href!}
      className={`block text-sm py-1.5 pr-3 rounded-md transition-colors ${
        isActive
          ? 'text-zinc-900 dark:text-white bg-zinc-100 dark:bg-zinc-700 font-medium'
          : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800'
      }`}
      style={{ paddingLeft: `${depth * 12 + 20}px` }}
    >
      {item.label}
    </Link>
  )
}

export default function Sidebar() {
  return (
    <aside className="w-60 shrink-0 h-screen sticky top-0 bg-zinc-50 dark:bg-zinc-900 border-r border-zinc-200 dark:border-zinc-800 flex flex-col">
      <div className="px-4 py-5 border-b border-zinc-200 dark:border-zinc-800">
        <div className="flex items-center justify-between">
          <Link href="/" className="text-lg font-bold text-zinc-900 dark:text-white tracking-tight">
            LLM Lab
          </Link>
          <ThemeToggle />
        </div>
        <p className="text-xs text-zinc-400 dark:text-zinc-500 mt-0.5">Learning playground</p>
      </div>
      <nav className="flex-1 overflow-y-auto py-4 space-y-1">
        {NAV_ITEMS.map((item) => (
          <NavNode key={item.label} item={item} />
        ))}
      </nav>
    </aside>
  )
}
