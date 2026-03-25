'use client'

import React from 'react'
import dynamic from 'next/dynamic'
import { Loader2 } from 'lucide-react'

// Wrap our reader client in a dynamic import to avoid SSR issues with pdfjs
const PdfReaderClient = dynamic(() => import('./PdfReaderClient'), {
  ssr: false,
  loading: () => (
    <div className="flex flex-col items-center justify-center h-[50vh] space-y-4">
      <Loader2 className="w-8 h-8 text-indigo-500 animate-spin" />
      <span className="text-zinc-500 font-medium text-sm">Loading PDF Engine...</span>
    </div>
  )
})

export default function EnhancedPDFReaderPage() {
  return <PdfReaderClient />
}
