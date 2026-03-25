'use client'

import { useEffect, useState } from 'react'
import { PanelLeftClose, PanelLeftOpen } from 'lucide-react'
import Sidebar from '@/components/layout/Sidebar'

const SIDEBAR_STATE_KEY = 'app-sidebar-collapsed'

export default function AppShell({ children }: { children: React.ReactNode }) {
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(() => {
    if (typeof window === 'undefined') return false
    return window.localStorage.getItem(SIDEBAR_STATE_KEY) === 'true'
  })

  useEffect(() => {
    window.localStorage.setItem(SIDEBAR_STATE_KEY, String(isSidebarCollapsed))
  }, [isSidebarCollapsed])

  return (
    <div className="flex min-h-screen">
      <div
        className={`relative shrink-0 transition-[width] duration-200 ease-out ${
          isSidebarCollapsed ? 'w-0' : 'w-60'
        }`}
      >
        <div
          className={`h-full overflow-hidden border-r border-zinc-200 dark:border-zinc-800 ${
            isSidebarCollapsed ? 'pointer-events-none opacity-0' : 'opacity-100'
          } transition-opacity duration-200`}
        >
          <Sidebar />
        </div>
        {!isSidebarCollapsed && (
          <button
            type="button"
            onClick={() => setIsSidebarCollapsed(true)}
            className="absolute right-3 top-1/2 z-50 inline-flex h-10 w-10 -translate-y-1/2 items-center justify-center rounded-xl border border-zinc-200 bg-white/92 text-zinc-700 shadow-sm backdrop-blur transition-colors hover:bg-zinc-100 dark:border-zinc-800 dark:bg-zinc-900/92 dark:text-zinc-200 dark:hover:bg-zinc-800"
            aria-label="Hide sidebar"
            title="Hide sidebar"
          >
            <PanelLeftClose className="h-5 w-5" />
          </button>
        )}
      </div>

      <main className="relative flex-1 overflow-auto">
        {isSidebarCollapsed && (
          <button
            type="button"
            onClick={() => setIsSidebarCollapsed(false)}
            className="fixed left-0 top-1/2 z-50 inline-flex h-12 w-11 -translate-y-1/2 items-center justify-center rounded-r-xl border border-l-0 border-zinc-200 bg-white/92 text-zinc-700 shadow-sm backdrop-blur transition-colors hover:bg-zinc-100 dark:border-zinc-800 dark:bg-zinc-900/92 dark:text-zinc-200 dark:hover:bg-zinc-800"
            aria-label="Show sidebar"
            title="Show sidebar"
          >
            <PanelLeftOpen className="h-5 w-5" />
          </button>
        )}
        {children}
      </main>
    </div>
  )
}
