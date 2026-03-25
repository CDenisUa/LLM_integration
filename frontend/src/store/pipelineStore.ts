// Core
import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export interface ProductData {
  name: string
  type: string
  audience: string
  brandStyle: string
  description: string
}

export interface VisualsData {
  imagePrompt: string
  images: string[]
  selectedImage: string
}

export interface SeoData {
  h1: string
  title: string
  metaDescription: string
  altText: string
}

export interface ContentData {
  heroHeadline: string
  heroSubtext: string
  features: string[]
  ctaText: string
  testimonial: string
}

export interface StylesData {
  primaryColor: string
  secondaryColor: string
  fontStyle: string
  animationStyle: string
}

interface PipelineStore {
  currentStep: number
  product: ProductData
  visuals: VisualsData
  seo: SeoData
  content: ContentData
  styles: StylesData
  finalHtml: string
  setStep: (step: number) => void
  setProduct: (data: Partial<ProductData>) => void
  setVisuals: (data: Partial<VisualsData>) => void
  setSeo: (data: Partial<SeoData>) => void
  setContent: (data: Partial<ContentData>) => void
  setStyles: (data: Partial<StylesData>) => void
  setFinalHtml: (html: string) => void
  reset: () => void
}

const defaultState = {
  currentStep: 0,
  product: { name: '', type: '', audience: '', brandStyle: '', description: '' },
  visuals: { imagePrompt: '', images: [], selectedImage: '' },
  seo: { h1: '', title: '', metaDescription: '', altText: '' },
  content: { heroHeadline: '', heroSubtext: '', features: [], ctaText: '', testimonial: '' },
  styles: { primaryColor: '#6366f1', secondaryColor: '#f59e0b', fontStyle: 'modern', animationStyle: 'smooth' },
  finalHtml: '',
}

export const usePipelineStore = create<PipelineStore>()(
  persist(
    (set) => ({
      ...defaultState,
      setStep: (step) => set({ currentStep: step }),
      setProduct: (data) => set((s) => ({ product: { ...s.product, ...data } })),
      setVisuals: (data) => set((s) => ({ visuals: { ...s.visuals, ...data } })),
      setSeo: (data) => set((s) => ({ seo: { ...s.seo, ...data } })),
      setContent: (data) => set((s) => ({ content: { ...s.content, ...data } })),
      setStyles: (data) => set((s) => ({ styles: { ...s.styles, ...data } })),
      setFinalHtml: (html) => set({ finalHtml: html }),
      reset: () => set(defaultState),
    }),
    { name: 'pipeline-store' }
  )
)
