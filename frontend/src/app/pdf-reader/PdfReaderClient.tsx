'use client'

import React, { useState, useEffect, useRef, useCallback } from 'react'
import { Document, Page, pdfjs } from 'react-pdf'
import 'react-pdf/dist/Page/AnnotationLayer.css'
import 'react-pdf/dist/Page/TextLayer.css'
import { Play, Square, ChevronLeft, ChevronRight, Upload, ZoomIn, ZoomOut, Loader2, BookOpen, ArrowLeft, Pencil, Expand, Shrink } from 'lucide-react'
import { API_URL } from '@/consts/api'

if (typeof window !== 'undefined') {
  pdfjs.GlobalWorkerOptions.workerSrc = `//unpkg.com/pdfjs-dist@${pdfjs.version}/build/pdf.worker.min.mjs`
}

type TextMapping = {
  index: number
  str: string
  isSpoken: boolean
  lineId: string | null
  x: number
  y: number
  fontSize: number
  left: number
  top: number
  width: number
  height: number
}

type ReaderLine = {
  id: string
  text: string
  itemIndexes: number[]
}

type OverlayRect = {
  lineId: string
  lineIndex: number
  left: number
  top: number
  width: number
  height: number
}

type WordRect = {
  itemIndex: number
  lineId: string
  lineIndex: number
  left: number
  top: number
  width: number
  height: number
}

type SpeechLineRange = {
  lineId: string
  lineIndex: number
  startChar: number
  endChar: number
}

type SortableTextItem = {
  index: number
  str: string
  isSpoken: boolean
  x: number
  y: number
  fontSize: number
  left: number
  top: number
  width: number
  height: number
}

type TtsResponse = {
  audio_base64: string
  alignment?: {
    character_start_times_seconds: number[]
    character_end_times_seconds: number[]
  }
}

type PdfMetadata = {
  id: string;
  filename: string;
  uploaded_at: number;
  has_cover: boolean;
}

type ReaderProgress = {
  pageNumber: number
  lineIndex: number
}

const READER_PROGRESS_PREFIX = 'pdf-reader-progress:'
const READER_SESSION_KEY = 'pdf-reader-session'

const getReaderProgressKey = (bookId: string) => `${READER_PROGRESS_PREFIX}${bookId}`
const escapeHtml = (value: string) =>
  value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')

const generateThumbnail = async (file: File): Promise<Blob | null> => {
  try {
    const url = URL.createObjectURL(file);
    const loadingTask = pdfjs.getDocument(url);
    const pdf = await loadingTask.promise;
    const page = await pdf.getPage(1);
    const viewport = page.getViewport({ scale: 0.5 });
    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d');
    if (!context) return null;
    canvas.height = viewport.height;
    canvas.width = viewport.width;
    // @ts-expect-error: pdfjs types vary by version
    await page.render({ canvasContext: context, viewport }).promise;
    
    return new Promise((resolve) => {
        canvas.toBlob((blob) => {
            resolve(blob);
        }, 'image/jpeg', 0.8);
    });
  } catch(e) {
    console.error("Cover generation failed", e);
    return null;
  }
}

export default function PdfReaderClient() {
  const [viewMode, setViewMode] = useState<'loading' | 'upload' | 'library' | 'reader'>('loading')
  const [library, setLibrary] = useState<PdfMetadata[]>([])
  const [isUploading, setIsUploading] = useState(false)
  
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editTitle, setEditTitle] = useState('')

  const [currentBookId, setCurrentBookId] = useState<string | null>(null)
  const [file, setFile] = useState<File | string | null>(null)
  const [numPages, setNumPages] = useState<number>(0)
  const [pageNumber, setPageNumber] = useState(1)
  const [scale, setScale] = useState(1.2)
  
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [pdfProxy, setPdfProxy] = useState<any>(null)
  const [mappings, setMappings] = useState<TextMapping[]>([])
  const [pageLines, setPageLines] = useState<ReaderLine[]>([])
  const [overlayRects, setOverlayRects] = useState<OverlayRect[]>([])
  const [wordRects, setWordRects] = useState<WordRect[]>([])
  const [activeLineId, setActiveLineId] = useState<string | null>(null)
  const [currentLineIndex, setCurrentLineIndex] = useState(0)
  const [isPlaying, setIsPlaying] = useState(false)
  const [isLoadingAudio, setIsLoadingAudio] = useState(false)
  
  const audioRef = useRef<HTMLAudioElement | null>(null)
  const animFrameRef = useRef<number | null>(null)
  const objectUrlRef = useRef<string | null>(null)
  const pendingResumeRef = useRef<ReaderProgress | null>(null)
  const pendingAutoPlayRef = useRef(false)
  const currentLineIndexRef = useRef(0)
  const activeLineIdRef = useRef<string | null>(null)
  const requestAbortRef = useRef<AbortController | null>(null)
  const requestSequenceRef = useRef(0)
  const ttsCacheRef = useRef<Map<string, TtsResponse>>(new Map())
  const readerViewportRef = useRef<HTMLDivElement | null>(null)
  const pageSurfaceRef = useRef<HTMLDivElement | null>(null)
  const pageGestureActiveRef = useRef(false)
  const pageGestureTimeoutRef = useRef<number | null>(null)
  const currentPageRef = useRef(1)
  const numPagesRef = useRef(0)
  const [isFullscreen, setIsFullscreen] = useState(false)
  const [fullscreenScale, setFullscreenScale] = useState<number | null>(null)
  const [pageFlipDirection, setPageFlipDirection] = useState<'forward' | 'backward'>('forward')
  const renderScale = isFullscreen && fullscreenScale ? fullscreenScale : scale

  useEffect(() => {
    fetch(`${API_URL}/api/pdf`)
      .then(res => {
         if (!res.ok) throw new Error("Failed fetching library");
         return res.json()
      })
      .then((data: PdfMetadata[]) => {
        setLibrary(data)
        const savedSession = typeof window !== 'undefined'
          ? window.localStorage.getItem(READER_SESSION_KEY)
          : null

        if (savedSession) {
          try {
            const parsed = JSON.parse(savedSession) as { bookId?: string }
            const matchingBook = data.find((book) => book.id === parsed.bookId)
            if (matchingBook) {
              openBook(matchingBook.id)
              return
            }
          } catch {
            // ignore malformed persisted session
          }
        }

        if (data.length > 0) setViewMode('library')
        else setViewMode('upload')
      })
      .catch(err => {
        console.error("Failed to fetch library", err);
        setViewMode('upload');
      });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const onFileChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFile = event.target.files?.[0]
    if (selectedFile) {
      setIsUploading(true)
      const thumbnailBlob = await generateThumbnail(selectedFile)
      
      const formData = new FormData()
      formData.append('file', selectedFile)
      if (thumbnailBlob) {
        formData.append('cover', thumbnailBlob, 'cover.jpg')
      }

      try {
        const res = await fetch(`${API_URL}/api/pdf/upload`, {
          method: 'POST',
          body: formData
        })
        if (!res.ok) {
           const errText = await res.text();
           console.error("Upload error text:", errText);
           throw new Error("Upload failed");
        }
        const meta: PdfMetadata = await res.json()
        setLibrary([meta, ...library])
        setCurrentBookId(meta.id)
        if (typeof window !== 'undefined') {
          window.localStorage.setItem(READER_SESSION_KEY, JSON.stringify({ bookId: meta.id }))
        }
        setFile(`${API_URL}/api/pdf/files/${meta.id}.pdf`)
        setPageNumber(1)
        setCurrentLineIndex(0)
        stopAudio()
        setViewMode('reader')
      } catch (err) {
        console.error("Upload process failed:", err)
      } finally {
        setIsUploading(false)
      }
    }
  }

  const saveTitle = async (id: string) => {
    if (!editTitle.trim()) { setEditingId(null); return; }
    try {
      const res = await fetch(`${API_URL}/api/pdf/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ filename: editTitle })
      })
      if (res.ok) {
        const updated = await res.json()
        setLibrary(library.map(b => b.id === id ? updated : b))
      }
    } catch(e) { console.error(e) }
    setEditingId(null)
  }

  const openBook = (id: string) => {
    setCurrentBookId(id)
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(READER_SESSION_KEY, JSON.stringify({ bookId: id }))
    }
    setFile(`${API_URL}/api/pdf/files/${id}.pdf`)
    const savedProgress = typeof window !== 'undefined'
      ? window.localStorage.getItem(getReaderProgressKey(id))
      : null

    if (savedProgress) {
      try {
        const parsed = JSON.parse(savedProgress) as ReaderProgress
        const safePageNumber = Math.max(parsed.pageNumber || 1, 1)
        const safeLineIndex = Math.max(parsed.lineIndex || 0, 0)
        setPageNumber(safePageNumber)
        setCurrentLineIndex(safeLineIndex)
        pendingResumeRef.current = { pageNumber: safePageNumber, lineIndex: safeLineIndex }
      } catch {
        setPageNumber(1)
        setCurrentLineIndex(0)
        pendingResumeRef.current = { pageNumber: 1, lineIndex: 0 }
      }
    } else {
      setPageNumber(1)
      setCurrentLineIndex(0)
      pendingResumeRef.current = { pageNumber: 1, lineIndex: 0 }
    }
    stopAudio()
    setViewMode('reader')
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const onDocumentLoadSuccess = (pdf: any) => {
    setNumPages(pdf.numPages)
    setPdfProxy(pdf) 
  }

  const saveProgress = useCallback((progress: ReaderProgress) => {
    if (!currentBookId || typeof window === 'undefined') return

    window.localStorage.setItem(
      getReaderProgressKey(currentBookId),
      JSON.stringify(progress)
    )
  }, [currentBookId])

  const clearReaderSession = useCallback(() => {
    if (typeof window === 'undefined') return
    window.localStorage.removeItem(READER_SESSION_KEY)
  }, [])

  useEffect(() => {
    currentLineIndexRef.current = currentLineIndex
  }, [currentLineIndex])

  useEffect(() => {
    activeLineIdRef.current = activeLineId
  }, [activeLineId])

  useEffect(() => {
    currentPageRef.current = pageNumber
  }, [pageNumber])

  useEffect(() => {
    numPagesRef.current = numPages
  }, [numPages])

  useEffect(() => {
    if (!pdfProxy || viewMode !== 'reader') return

    const extractText = async () => {
      try {
        const page = await pdfProxy.getPage(pageNumber)
        const textContent = await page.getTextContent()

        const viewport = page.getViewport({ scale: renderScale })

        const sortableItems: Array<SortableTextItem & { left: number; top: number; width: number; height: number }> = textContent.items
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          .map((item: any, index: number) => {
            const strTrimmed = item.str.trim()
            const tx = pdfjs.Util.transform(viewport.transform, item.transform)
            const fontSize = Math.hypot(tx[2], tx[3])
            const x = Number(tx[4] || 0)
            const y = Number(tx[5] || 0)
            const width = Math.max(Number(item.width || 0) * renderScale, fontSize * Math.max(strTrimmed.length, 1) * 0.35)
            const height = Math.max(fontSize, Number(item.height || 0) * renderScale || 0)
            const left = x
            const top = y - height * 0.8

            let isSpoken = true
            if (!strTrimmed) isSpoken = false

            return {
              index,
              str: item.str,
              isSpoken,
              x,
              y,
              fontSize,
              left,
              top,
              width,
              height,
            }
          })
          .sort((a: SortableTextItem, b: SortableTextItem) => {
            if (Math.abs(a.top - b.top) > 3) return a.top - b.top
            return a.left - b.left
          })

        const lines: ReaderLine[] = []
        const lineBuckets: Array<{ y: number; tolerance: number; items: SortableTextItem[] }> = []

        for (const item of sortableItems) {
          if (!item.str.trim()) continue

          const itemTolerance = Math.max(3, item.height * 0.45)
          const bucket = lineBuckets.find(
            ({ y, tolerance }) => Math.abs(y - item.top) <= Math.max(tolerance, itemTolerance)
          )

          if (bucket) {
            bucket.items.push(item)
            bucket.y = (bucket.y * (bucket.items.length - 1) + item.top) / bucket.items.length
            bucket.tolerance = Math.max(bucket.tolerance, itemTolerance)
          } else {
            lineBuckets.push({ y: item.top, tolerance: itemTolerance, items: [item] })
          }
        }

        lineBuckets
          .sort((a, b) => a.y - b.y)
          .forEach((bucket, bucketIndex) => {
            const spokenItems = bucket.items
              .sort((a, b) => a.left - b.left)

            const text = spokenItems.reduce((accumulator, item, itemIndex) => {
              const normalizedText = item.str.replace(/\s+/g, ' ').trim()
              if (!normalizedText) return accumulator

              if (itemIndex === 0) return normalizedText

              const previousItem = spokenItems[itemIndex - 1]
              const gap = item.left - (previousItem.left + previousItem.width)
              const joiner = gap > previousItem.fontSize * 1.2 ? '  ' : ' '
              return `${accumulator}${joiner}${normalizedText}`
            }, '').trim()

            if (!text) return

            lines.push({
              id: `page-${pageNumber}-line-${bucketIndex}`,
              text,
              itemIndexes: spokenItems.map((item) => item.index),
            })
          })

        const lineIdByItemIndex = new Map<number, string>()
        lines.forEach((line) => {
          line.itemIndexes.forEach((itemIndex) => {
            lineIdByItemIndex.set(itemIndex, line.id)
          })
        })

        const newMappings = Array.from({ length: textContent.items.length }, (_, index) => {
          const sortableItem = sortableItems.find((item) => item.index === index)
          if (!sortableItem) {
            return {
              index,
              str: '',
              isSpoken: false,
              lineId: null,
              x: 0,
              y: 0,
              fontSize: 0,
              left: 0,
              top: 0,
              width: 0,
              height: 0,
            }
          }

          return {
            ...sortableItem,
            lineId: lineIdByItemIndex.get(index) ?? null,
          }
        })

        setMappings(newMappings)
        setPageLines(lines)

        const lineIndexById = new Map(lines.map((line, index) => [line.id, index]))
        const nextWordRects: WordRect[] = newMappings
          .filter((mapping) => mapping.lineId && mapping.width > 0 && mapping.height > 0)
          .map((mapping) => ({
            itemIndex: mapping.index,
            lineId: mapping.lineId!,
            lineIndex: lineIndexById.get(mapping.lineId!) ?? 0,
            left: mapping.left,
            top: mapping.top,
            width: mapping.width,
            height: mapping.height,
          }))

        const lineRectMap = new Map<string, OverlayRect>()
        nextWordRects.forEach((rect) => {
          const existingRect = lineRectMap.get(rect.lineId)
          if (!existingRect) {
            lineRectMap.set(rect.lineId, {
              lineId: rect.lineId,
              lineIndex: rect.lineIndex,
              left: rect.left,
              top: rect.top,
              width: rect.width,
              height: rect.height,
            })
            return
          }

          const right = Math.max(existingRect.left + existingRect.width, rect.left + rect.width)
          const bottom = Math.max(existingRect.top + existingRect.height, rect.top + rect.height)
          existingRect.left = Math.min(existingRect.left, rect.left)
          existingRect.top = Math.min(existingRect.top, rect.top)
          existingRect.width = right - existingRect.left
          existingRect.height = bottom - existingRect.top
        })

        setWordRects(nextWordRects)
        setOverlayRects(Array.from(lineRectMap.values()).sort((a, b) => a.lineIndex - b.lineIndex))

        const pendingProgress = pendingResumeRef.current
        const nextLineIndex = pendingProgress && pendingProgress.pageNumber === pageNumber
          ? Math.min(pendingProgress.lineIndex, Math.max(lines.length - 1, 0))
          : Math.min(currentLineIndexRef.current, Math.max(lines.length - 1, 0))

        setCurrentLineIndex(nextLineIndex)
        setActiveLineId(lines[nextLineIndex]?.id ?? null)
        saveProgress({ pageNumber, lineIndex: nextLineIndex })

        if (pendingProgress && pendingProgress.pageNumber === pageNumber) {
          pendingResumeRef.current = null
        }
      } catch (err) {
        console.error('Error extracting text:', err)
      }
    }

    extractText()
  }, [pageNumber, pdfProxy, renderScale, saveProgress, viewMode])

  const stopAudio = useCallback(() => {
    if (requestAbortRef.current) {
      requestAbortRef.current.abort()
      requestAbortRef.current = null
    }
    if (audioRef.current) {
      audioRef.current.pause()
      audioRef.current.src = ''
      audioRef.current = null
    }
    if (animFrameRef.current) {
      cancelAnimationFrame(animFrameRef.current)
      animFrameRef.current = null
    }
    if (objectUrlRef.current) {
      URL.revokeObjectURL(objectUrlRef.current)
      objectUrlRef.current = null
    }
    setIsPlaying(false)
    setIsLoadingAudio(false)
  }, [])

  useEffect(() => {
    return () => stopAudio()
  }, [stopAudio])

  useEffect(() => {
    return () => {
      if (pageGestureTimeoutRef.current !== null) {
        window.clearTimeout(pageGestureTimeoutRef.current)
      }
    }
  }, [])

  useEffect(() => {
    const handleFullscreenChange = () => {
      setIsFullscreen(document.fullscreenElement === readerViewportRef.current)
    }

    document.addEventListener('fullscreenchange', handleFullscreenChange)
    return () => document.removeEventListener('fullscreenchange', handleFullscreenChange)
  }, [])

  useEffect(() => {
    if (!isFullscreen || !pdfProxy || !readerViewportRef.current) {
      setFullscreenScale(null)
      return
    }

    let isDisposed = false
    const container = readerViewportRef.current

    const updateFullscreenScale = async () => {
      try {
        const page = await pdfProxy.getPage(pageNumber)
        const viewport = page.getViewport({ scale: 1 })
        const availableHeight = Math.max(container.clientHeight - 32, 200)
        const nextScale = availableHeight / viewport.height

        if (!isDisposed) {
          setFullscreenScale(nextScale)
        }
      } catch (error) {
        console.error('Failed to fit PDF page to fullscreen height:', error)
      }
    }

    const resizeObserver = new ResizeObserver(() => {
      void updateFullscreenScale()
    })

    resizeObserver.observe(container)
    void updateFullscreenScale()

    return () => {
      isDisposed = true
      resizeObserver.disconnect()
    }
  }, [isFullscreen, pageNumber, pdfProxy])

  const buildLineText = useCallback((line: ReaderLine, startItemIndex?: number) => {
    const orderedIndexes = startItemIndex === undefined
      ? line.itemIndexes
      : line.itemIndexes.slice(Math.max(line.itemIndexes.indexOf(startItemIndex), 0))

    return orderedIndexes.reduce((accumulator, itemIndex, itemPosition) => {
      const mapping = mappings[itemIndex]
      const normalizedText = mapping?.str.replace(/\s+/g, ' ').trim() ?? ''
      if (!normalizedText) return accumulator

      if (itemPosition === 0) return normalizedText

      const previousMapping = mappings[orderedIndexes[itemPosition - 1]]
      const previousText = previousMapping?.str.replace(/\s+/g, ' ').trim() ?? ''
      const previousEndsWithHyphen = /[-\u2010\u2011]$/.test(previousText)
      const nextStartsLowercase = /^[a-zа-яё]/u.test(normalizedText)

      if (previousEndsWithHyphen && nextStartsLowercase) {
        return `${accumulator.slice(0, -1)}${normalizedText}`
      }

      const gap = (mapping?.x ?? 0) - ((previousMapping?.x ?? 0) + Math.max(previousMapping?.fontSize ?? 1, 1))
      const joiner = gap > Math.max(previousMapping?.fontSize ?? 1, 1) * 1.2 ? '  ' : ' '
      return `${accumulator}${joiner}${normalizedText}`
    }, '').trim()
  }, [mappings])

  const buildSpeechPayload = useCallback((startLineIndex: number, startItemIndex?: number) => {
    const ranges: SpeechLineRange[] = []
    let text = ''

    for (let lineIndex = startLineIndex; lineIndex < pageLines.length; lineIndex += 1) {
      const line = pageLines[lineIndex]
      const trimmedLine = buildLineText(
        line,
        lineIndex === startLineIndex ? startItemIndex : undefined
      )
      if (!trimmedLine) continue

      const previousChar = text.slice(-1)
      const nextLineStartsLowercase = /^[a-zа-яё]/u.test(trimmedLine)
      const previousEndsWithHyphen = /[-\u2010\u2011]$/.test(text)
      const previousNeedsSpace = text.length > 0 && !/\s$/.test(text)

      if (previousEndsWithHyphen && nextLineStartsLowercase) {
        text = text.slice(0, -1)
      } else if (previousNeedsSpace) {
        const shouldTightJoin = previousChar === '' || /[\u2014(“"']/.test(previousChar)
        if (!shouldTightJoin) {
          text += ' '
        }
      }

      const startChar = text.length
      text += trimmedLine
      const endChar = text.length

      ranges.push({
        lineId: line.id,
        lineIndex,
        startChar,
        endChar,
      })
    }

    return { text, ranges }
  }, [buildLineText, pageLines])

  const startReadingFromLine = useCallback(async (lineIndex: number, startItemIndex?: number) => {
    const line = pageLines[lineIndex]
    if (!line) {
      if (pageNumber < numPages) {
        pendingResumeRef.current = { pageNumber: pageNumber + 1, lineIndex: 0 }
        pendingAutoPlayRef.current = true
        setPageNumber((p) => p + 1)
      }
      return
    }

    const { text, ranges } = buildSpeechPayload(lineIndex, startItemIndex)
    if (!text.trim() || ranges.length === 0) return

    stopAudio()

    const requestId = requestSequenceRef.current + 1
    requestSequenceRef.current = requestId
    const abortController = new AbortController()
    requestAbortRef.current = abortController

    setIsLoadingAudio(true)
    currentLineIndexRef.current = lineIndex
    activeLineIdRef.current = line.id
    setCurrentLineIndex(lineIndex)
    setActiveLineId(line.id)
    saveProgress({ pageNumber, lineIndex })

    try {
      let data: TtsResponse | undefined = ttsCacheRef.current.get(text)

      if (!data) {
        const res = await fetch(`${API_URL}/api/tts`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ text }),
          signal: abortController.signal,
        })

        if (!res.ok) throw new Error(await res.text())

        data = await res.json() as TtsResponse
        ttsCacheRef.current.set(text, data)
      }

      if (requestSequenceRef.current !== requestId || abortController.signal.aborted) {
        return
      }

      if (!data) {
        throw new Error('TTS response is empty')
      }
      
      const binaryString = window.atob(data.audio_base64)
      const len = binaryString.length
      const bytes = new Uint8Array(len)
      for (let i = 0; i < len; i++) {
        bytes[i] = binaryString.charCodeAt(i)
      }
      const blob = new Blob([bytes.buffer], { type: 'audio/mp3' })
      const url = URL.createObjectURL(blob)
      objectUrlRef.current = url

      if (requestSequenceRef.current !== requestId || abortController.signal.aborted) {
        URL.revokeObjectURL(url)
        if (objectUrlRef.current === url) {
          objectUrlRef.current = null
        }
        return
      }

      const audio = new Audio(url)
      audioRef.current = audio
      requestAbortRef.current = null

      const starts = data.alignment?.character_start_times_seconds ?? []
      const ends = data.alignment?.character_end_times_seconds ?? []

      const syncHighlightedLine = () => {
        if (!audioRef.current) return

        const currentTime = audioRef.current.currentTime
        let currentCharIndex = -1

        for (let i = 0; i < starts.length; i += 1) {
          if (currentTime >= starts[i] && currentTime <= ends[i]) {
            currentCharIndex = i
            break
          }
        }

        if (currentCharIndex !== -1) {
          const activeRange = ranges.find(
            (range) => currentCharIndex >= range.startChar && currentCharIndex < range.endChar
          )

          if (activeRange) {
            if (activeLineIdRef.current !== activeRange.lineId) {
              activeLineIdRef.current = activeRange.lineId
              currentLineIndexRef.current = activeRange.lineIndex
              setActiveLineId(activeRange.lineId)
              setCurrentLineIndex(activeRange.lineIndex)
              saveProgress({ pageNumber, lineIndex: activeRange.lineIndex })
            }
          }
        }

        animFrameRef.current = requestAnimationFrame(syncHighlightedLine)
      }

      audio.onplay = () => {
        setIsPlaying(true)
        setIsLoadingAudio(false)
        const anchorId = line.itemIndexes[0]
        if (anchorId !== undefined) {
          const el = document.getElementById(`text-chunk-${anchorId}`)
          if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' })
        }
        animFrameRef.current = requestAnimationFrame(syncHighlightedLine)
      }

      audio.onended = () => {
        const lastRange = ranges[ranges.length - 1]
        if (lastRange) {
          currentLineIndexRef.current = lastRange.lineIndex
          activeLineIdRef.current = lastRange.lineId
          setCurrentLineIndex(lastRange.lineIndex)
          setActiveLineId(lastRange.lineId)
          saveProgress({ pageNumber, lineIndex: lastRange.lineIndex })
        }

        if (pageNumber < numPages) {
          pendingResumeRef.current = { pageNumber: pageNumber + 1, lineIndex: 0 }
          pendingAutoPlayRef.current = true
          setPageNumber((p) => p + 1)
        } else {
          saveProgress({ pageNumber, lineIndex })
          stopAudio()
        }
      }
      audio.onerror = () => stopAudio()

      await audio.play()

    } catch (e) {
      if (abortController.signal.aborted) {
        return
      }
      console.error('TTS error:', e)
      stopAudio()
    }
  }, [buildSpeechPayload, numPages, pageLines, pageNumber, saveProgress, stopAudio])

  useEffect(() => {
    if (!pendingAutoPlayRef.current) return
    if (pageLines.length === 0) return

    pendingAutoPlayRef.current = false
    const nextLineIndex = Math.min(currentLineIndex, Math.max(pageLines.length - 1, 0))
    const timeoutId = window.setTimeout(() => {
      void startReadingFromLine(nextLineIndex)
    }, 150)

    return () => window.clearTimeout(timeoutId)
  }, [currentLineIndex, pageLines, startReadingFromLine])

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const customTextRenderer = ({ str }: any) => escapeHtml(str)

  const bookProgress = numPages > 0
    ? Math.min(
        100,
        Math.round(
          (((pageNumber - 1) + (pageLines.length > 0 ? currentLineIndex / pageLines.length : 0)) / numPages) * 100
        )
      )
    : 0

  const toggleFullscreen = useCallback(async () => {
    const viewport = readerViewportRef.current
    if (!viewport) return

    if (document.fullscreenElement === viewport) {
      await document.exitFullscreen()
      return
    }

    await viewport.requestFullscreen()
  }, [])

  const goToPage = useCallback((nextPageNumber: number, direction: 'forward' | 'backward') => {
    const safePageNumber = Math.min(Math.max(nextPageNumber, 1), Math.max(numPages, 1))
    setPageFlipDirection(direction)
    setPageNumber(safePageNumber)
    setCurrentLineIndex(0)
    saveProgress({ pageNumber: safePageNumber, lineIndex: 0 })
    stopAudio()
  }, [numPages, saveProgress, stopAudio])

  const handlePageSurfaceWheel = useCallback((event: React.WheelEvent<HTMLDivElement>) => {
    if (viewMode !== 'reader' || numPagesRef.current <= 1) return

    if (pageGestureTimeoutRef.current !== null) {
      window.clearTimeout(pageGestureTimeoutRef.current)
    }

    pageGestureTimeoutRef.current = window.setTimeout(() => {
      pageGestureActiveRef.current = false
      pageGestureTimeoutRef.current = null
    }, 280)

    if (Math.abs(event.deltaY) < 4) return

    if (pageGestureActiveRef.current) {
      event.preventDefault()
      return
    }

    const currentPage = currentPageRef.current
    const totalPages = numPagesRef.current

    if (event.deltaY > 0 && currentPage < totalPages) {
      event.preventDefault()
      pageGestureActiveRef.current = true
      goToPage(currentPage + 1, 'forward')
      return
    }

    if (event.deltaY < 0 && currentPage > 1) {
      event.preventDefault()
      pageGestureActiveRef.current = true
      goToPage(currentPage - 1, 'backward')
    }
  }, [goToPage, viewMode])

  return (
    <div className={`flex flex-col h-full ${viewMode !== 'reader' ? 'max-w-6xl mx-auto p-4 md:p-8 space-y-6 w-full' : 'w-full'}`}>
      {viewMode !== 'reader' && (
        <header className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold tracking-tight text-zinc-900 dark:text-zinc-50">Your Library</h1>
            <p className="text-zinc-600 dark:text-zinc-400">Select a book to read or upload a new one.</p>
          </div>
          {viewMode === 'library' && (
             <button onClick={() => setViewMode('upload')} className="flex items-center gap-2 bg-black dark:bg-white text-white dark:text-black px-4 py-2 rounded-xl font-medium shadow-sm hover:opacity-90 transition-opacity">
               <Upload className="w-4 h-4" /> Upload PDF
             </button>
          )}
        </header>
      )}

      {viewMode === 'loading' && (
        <div className="flex-1 flex items-center justify-center">
           <Loader2 className="w-8 h-8 animate-spin text-zinc-500" />
        </div>
      )}

      {viewMode === 'library' && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-6">
          {library.map(book => (
            <div 
              key={book.id} 
              className="group flex flex-col items-center p-4 border border-zinc-200 dark:border-zinc-800 rounded-2xl bg-white dark:bg-zinc-900 hover:border-indigo-500 hover:shadow-md transition-all relative"
            >
              <div onClick={() => openBook(book.id)} className="cursor-pointer w-full aspect-[3/4] bg-zinc-100 dark:bg-zinc-800 rounded-lg flex items-center justify-center mb-3 group-hover:bg-indigo-50 dark:group-hover:bg-indigo-900/20 transition-colors overflow-hidden relative">
                {book.has_cover ? (
                  // eslint-disable-next-line
                  <img src={`${API_URL}/api/pdf/covers/${book.id}.jpg`} className="w-full h-full object-cover" alt={book.filename} />
                ) : (
                  <BookOpen className="w-12 h-12 text-zinc-400 group-hover:text-indigo-500 transition-colors" />
                )}
               </div>
              
              {editingId === book.id ? (
                <input 
                  autoFocus
                  value={editTitle}
                  onChange={(e) => setEditTitle(e.target.value)}
                  onBlur={() => saveTitle(book.id)}
                  onKeyDown={(e) => e.key === 'Enter' && saveTitle(book.id)}
                  className="text-sm font-medium text-center w-full px-2 py-1 outline-none border border-indigo-500 rounded bg-transparent text-zinc-900 dark:text-zinc-100"
                />
              ) : (
                <div className="flex items-center justify-center w-full gap-2 pl-4">
                  <h3 className="text-sm font-medium text-center line-clamp-2 text-zinc-900 dark:text-zinc-100 cursor-pointer flex-1 break-all" onClick={() => openBook(book.id)} title={book.filename}>
                    {book.filename.replace(/\.pdf$/i, '')}
                  </h3>
                  <button onClick={(e) => { e.stopPropagation(); setEditingId(book.id); setEditTitle(book.filename.replace(/\.pdf$/i, '')); }} className="opacity-0 group-hover:opacity-100 p-1 text-zinc-400 hover:text-indigo-500 transition-opacity">
                     <Pencil className="w-4 h-4" />
                  </button>
                </div>
              )}
              <p className="text-xs text-zinc-500 mt-2">{new Date(book.uploaded_at * 1000).toLocaleDateString()}</p>
            </div>
          ))}
        </div>
      )}

      {viewMode === 'upload' && (
        <div className="flex-1 flex flex-col items-center justify-center p-12 border-2 border-dashed border-zinc-300 dark:border-zinc-700 rounded-2xl bg-zinc-50 dark:bg-zinc-900/50 hover:bg-zinc-100 dark:hover:bg-zinc-900 transition-colors">
          {isUploading ? (
             <div className="flex flex-col items-center text-indigo-600">
               <Loader2 className="w-10 h-10 mb-4 animate-spin" />
               <p className="font-medium text-lg text-zinc-900 dark:text-zinc-100">Uploading and processing...</p>
             </div>
          ) : (
            <>
              <Upload className="w-10 h-10 text-zinc-400 mb-4" />
              <h3 className="text-lg font-medium text-zinc-900 dark:text-zinc-100 mb-2">Upload a PDF Book</h3>
              <p className="text-zinc-500 text-sm mb-6 text-center max-w-sm">We securely store your PDFs locally to save API costs.</p>
              <label className="cursor-pointer bg-black dark:bg-white text-white dark:text-black px-6 py-3 rounded-xl font-medium shadow-sm hover:opacity-90 transition-opacity">
                Select PDF File
                <input type="file" accept=".pdf" className="hidden" onChange={onFileChange} />
              </label>
              {library.length > 0 && (
                <button onClick={() => { clearReaderSession(); setViewMode('library') }} className="mt-6 text-sm text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors flex items-center gap-1">
                   <ArrowLeft className="w-4 h-4" /> Back to Library
                </button>
              )}
            </>
          )}
        </div>
      )}

      {viewMode === 'reader' && (
        <div className="flex-1 flex flex-col bg-zinc-100 dark:bg-zinc-950 overflow-hidden">
          {/* PDF Viewer Area */}
          <div
            ref={readerViewportRef}
            tabIndex={-1}
            className={`flex-1 overflow-auto bg-zinc-200/50 dark:bg-[#121212] flex justify-center p-2 md:p-4 ${
              isFullscreen ? 'pdf-reader-fullscreen' : ''
            }`}
          >
            <Document
              file={file}
              onLoadSuccess={onDocumentLoadSuccess}
              loading={<div className="text-zinc-500 flex items-center gap-2 mt-10"><Loader2 className="w-5 h-5 animate-spin" /> Loading Document...</div>}
              className="flex flex-col items-center"
            >
              <div
                ref={pageSurfaceRef}
                key={`${pageNumber}-${pageFlipDirection}`}
                onWheelCapture={handlePageSurfaceWheel}
                className={`relative shadow-lg transform-gpu border border-black/5 dark:border-white/5 transition-transform origin-top scroll-smooth ${
                  pageFlipDirection === 'forward' ? 'pdf-page-flip-forward' : 'pdf-page-flip-backward'
                }`}
              >
                <Page 
                  pageNumber={pageNumber} 
                  scale={renderScale}
                  renderTextLayer={true}
                  renderAnnotationLayer={true}
                  customTextRenderer={customTextRenderer}
                  loading={<div className="h-[800px] w-[600px] bg-white dark:bg-zinc-900 animate-pulse" />}
                  className="bg-white dark:bg-zinc-900"
                />
                <div className="absolute inset-0 z-30 pointer-events-none">
                  {overlayRects.map((rect) => {
                    const isActive = rect.lineId === activeLineId
                    const isRead = rect.lineIndex < currentLineIndex

                    return (
                      <div
                        key={rect.lineId}
                        className={`absolute rounded-sm ${
                          isActive
                            ? 'bg-amber-300/85 ring-1 ring-amber-500'
                            : isRead
                              ? 'bg-amber-200/70'
                              : 'bg-transparent'
                        }`}
                        style={{
                          left: rect.left,
                          top: rect.top,
                          width: rect.width,
                          height: rect.height,
                        }}
                      />
                    )
                  })}
                </div>
                <div className="absolute inset-0 z-40">
                  {wordRects.map((rect) => (
                    <button
                      key={`${rect.itemIndex}-${rect.left}-${rect.top}`}
                      type="button"
                      onClick={() => void startReadingFromLine(rect.lineIndex, rect.itemIndex)}
                      className="absolute rounded-[2px] bg-transparent hover:bg-indigo-300/30 focus:bg-indigo-300/30 focus:outline-none"
                      style={{
                        left: rect.left,
                        top: rect.top,
                        width: rect.width,
                        height: rect.height,
                      }}
                      aria-label={`Read from word ${rect.itemIndex}`}
                      title="Click to read from this word"
                    />
                  ))}
                </div>
              </div>
            </Document>
          </div>

          <div className="px-4 py-3 bg-white dark:bg-zinc-900 border-t border-zinc-200 dark:border-zinc-800 z-10 w-full shrink-0 space-y-3">
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0 flex-1">
                <div className="flex items-center justify-between gap-3 text-xs font-medium text-zinc-500">
                  <span>Book progress</span>
                  <span>{bookProgress}%</span>
                </div>
                <div className="mt-1 h-2 rounded-full bg-zinc-200 dark:bg-zinc-800 overflow-hidden">
                  <div
                    className="h-full rounded-full bg-indigo-600 transition-all duration-300"
                    style={{ width: `${bookProgress}%` }}
                  />
                </div>
                <p className="mt-1 text-xs text-zinc-500">
                  Page {pageNumber} of {numPages || 1}
                  {pageLines.length > 0 ? ` • Line ${Math.min(currentLineIndex + 1, pageLines.length)} of ${pageLines.length}` : ''}
                </p>
              </div>
            </div>

            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex flex-wrap items-center gap-2 md:gap-4 shrink-0">
                <button
                  onClick={() => { setFile(null); stopAudio(); clearReaderSession(); setViewMode('library') }}
                  className="p-2 flex items-center gap-1 text-sm font-medium rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors mr-2 text-zinc-600 dark:text-zinc-300 shrink-0"
                  title="Back to Library"
                >
                  <ArrowLeft className="w-5 h-5" />
                  <span className="hidden md:inline">Library</span>
                </button>

                <div className="w-px h-6 bg-zinc-300 dark:bg-zinc-700 mx-1 md:mx-2" />

                <button
                  onClick={() => {
                    goToPage(pageNumber - 1, 'backward')
                  }}
                  disabled={pageNumber <= 1}
                  className="p-2 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 disabled:opacity-50 transition-colors"
                  title="Previous Page"
                >
                  <ChevronLeft className="w-5 h-5" />
                </button>
                <div className="flex items-center gap-2 text-sm font-medium">
                  <input
                    type="number"
                    min={1}
                    max={numPages || 1}
                    value={pageNumber}
                    onChange={(e) => {
                      const val = parseInt(e.target.value)
                      if (val >= 1 && val <= numPages) {
                        goToPage(val, val >= pageNumber ? 'forward' : 'backward')
                      }
                    }}
                    className="w-12 text-center p-1 rounded border border-zinc-300 dark:border-zinc-700 bg-transparent text-zinc-900 dark:text-zinc-100"
                  />
                  <span className="text-zinc-500 whitespace-nowrap">of {numPages}</span>
                </div>
                <button
                  onClick={() => {
                    goToPage(pageNumber + 1, 'forward')
                  }}
                  disabled={pageNumber >= numPages}
                  className="p-2 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 disabled:opacity-50 transition-colors"
                  title="Next Page"
                >
                  <ChevronRight className="w-5 h-5" />
                </button>
              </div>

              <div className="flex flex-wrap items-center gap-2 md:gap-4 shrink-0">
                <button
                  onClick={() => setScale(s => Math.max(s - 0.2, 0.5))}
                  className="p-2 hidden md:block rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-500 shrink-0"
                  title="Zoom Out"
                >
                  <ZoomOut className="w-5 h-5" />
                </button>
                <span className="text-sm font-medium text-zinc-500 hidden md:block shrink-0">{Math.round(scale * 100)}%</span>
                <button
                  onClick={() => setScale(s => Math.min(s + 0.2, 3))}
                  className="p-2 hidden md:block rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-500 shrink-0"
                  title="Zoom In"
                >
                  <ZoomIn className="w-5 h-5" />
                </button>

                <button
                  type="button"
                  onClick={() => void toggleFullscreen()}
                  className="flex items-center gap-2 rounded-lg border border-zinc-200 px-3 py-2 text-sm font-medium text-zinc-700 transition-colors hover:bg-zinc-100 dark:border-zinc-700 dark:text-zinc-200 dark:hover:bg-zinc-800"
                  title={isFullscreen ? 'Exit Fullscreen' : 'Open Fullscreen'}
                >
                  {isFullscreen ? <Shrink className="h-4 w-4" /> : <Expand className="h-4 w-4" />}
                  <span className="hidden sm:inline">{isFullscreen ? 'Exit Fullscreen' : 'Fullscreen'}</span>
                </button>

                <div className="w-px h-6 bg-zinc-300 dark:bg-zinc-700 mx-1 md:mx-2" />

                {!isPlaying && !isLoadingAudio ? (
                  <button
                    onClick={() => void startReadingFromLine(currentLineIndex)}
                    className="flex items-center gap-2 bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-lg font-medium shadow-sm transition-colors text-sm cursor-pointer shrink-0"
                  >
                    <Play className="w-4 h-4 fill-current" />
                    <span className="hidden sm:inline">Play From Here</span>
                  </button>
                ) : isLoadingAudio ? (
                  <button
                    disabled
                    className="flex items-center gap-2 bg-indigo-400 text-white px-4 py-2 rounded-lg font-medium shadow-sm transition-colors text-sm opacity-80 cursor-not-allowed shrink-0"
                  >
                    <Loader2 className="w-4 h-4 animate-spin" />
                    <span className="hidden sm:inline">Loading Audio...</span>
                  </button>
                ) : (
                  <button
                    onClick={stopAudio}
                    className="flex items-center gap-2 bg-rose-600 hover:bg-rose-700 text-white px-4 py-2 rounded-lg font-medium shadow-sm transition-colors text-sm cursor-pointer shrink-0"
                  >
                    <Square className="w-4 h-4 fill-current" />
                    <span className="hidden sm:inline">Stop</span>
                  </button>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
