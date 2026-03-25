// Consts
import { API_URL } from '@/consts/api'
// Types
import type { ProductData, SeoData, ContentData, StylesData } from '@/store/pipelineStore'

async function post<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${API_URL}/api/pipeline/${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  if (!res.ok) {
    const err = await res.json()
    throw new Error(err.error || 'Request failed')
  }
  return res.json()
}

export async function generateImagePrompt(product: ProductData): Promise<string> {
  const data = await post<{ prompt: string }>('image-prompt', {
    product_name: product.name,
    product_type: product.type,
    audience: product.audience,
    brand_style: product.brandStyle,
  })
  return data.prompt
}

export async function generateImages(prompt: string): Promise<string[]> {
  const data = await post<{ images: string[] }>('generate-image', { prompt })
  return data.images
}

export async function generateSeo(product: ProductData): Promise<SeoData> {
  const data = await post<{ h1: string; title: string; meta_description: string; alt_text: string }>('seo', {
    product_name: product.name,
    product_type: product.type,
    audience: product.audience,
    description: product.description,
  })
  return { h1: data.h1, title: data.title, metaDescription: data.meta_description, altText: data.alt_text }
}

export async function generateContent(product: ProductData): Promise<ContentData> {
  const data = await post<{
    hero_headline: string
    hero_subtext: string
    features: string[]
    cta_text: string
    testimonial: string
  }>('content', {
    product_name: product.name,
    product_type: product.type,
    audience: product.audience,
    brand_style: product.brandStyle,
    description: product.description,
  })
  return {
    heroHeadline: data.hero_headline,
    heroSubtext: data.hero_subtext,
    features: data.features,
    ctaText: data.cta_text,
    testimonial: data.testimonial,
  }
}

export async function generateStyles(product: ProductData): Promise<StylesData & { reasoning: string }> {
  const data = await post<{
    primary_color: string
    secondary_color: string
    font_style: string
    animation_style: string
    reasoning: string
  }>('styles', {
    product_name: product.name,
    brand_style: product.brandStyle,
    product_type: product.type,
  })
  return {
    primaryColor: data.primary_color,
    secondaryColor: data.secondary_color,
    fontStyle: data.font_style,
    animationStyle: data.animation_style,
    reasoning: data.reasoning,
  }
}

export async function assemblePage(
  product: ProductData,
  selectedImage: string,
  seo: SeoData,
  content: ContentData,
  styles: StylesData,
): Promise<string> {
  const data = await post<{ html: string }>('assemble', {
    product_name: product.name,
    selected_image: selectedImage,
    seo: { h1: seo.h1, title: seo.title, meta_description: seo.metaDescription, alt_text: seo.altText },
    content: {
      hero_headline: content.heroHeadline,
      hero_subtext: content.heroSubtext,
      features: content.features,
      cta_text: content.ctaText,
      testimonial: content.testimonial,
    },
    styles: {
      primary_color: styles.primaryColor,
      secondary_color: styles.secondaryColor,
      font_style: styles.fontStyle,
      animation_style: styles.animationStyle,
    },
  })
  return data.html
}
