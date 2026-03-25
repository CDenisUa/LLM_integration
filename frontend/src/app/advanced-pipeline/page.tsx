'use client'

// Core
import { useState } from 'react'
// Hooks
import { useTranslations } from '@/hooks/useTranslations'
// Store
import { usePipelineStore } from '@/store/pipelineStore'
// Services
import {
  generateImagePrompt,
  generateImages,
  generateSeo,
  generateContent,
  generateStyles,
  assemblePage,
} from '@/services/pipeline'

// ── Step indicator ────────────────────────────────────────────────────────────

function StepIndicator({ current, steps }: { current: number; steps: string[] }) {
  return (
    <div className="flex items-center gap-0 mb-8">
      {steps.map((step, i) => (
        <div key={i} className="flex items-center">
          <div className="flex flex-col items-center">
            <div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-semibold border-2 transition-colors ${
              i < current
                ? 'bg-zinc-900 dark:bg-white border-zinc-900 dark:border-white text-white dark:text-zinc-900'
                : i === current
                ? 'border-zinc-900 dark:border-white text-zinc-900 dark:text-white bg-transparent'
                : 'border-zinc-300 dark:border-zinc-700 text-zinc-400 dark:text-zinc-600'
            }`}>
              {i < current ? (
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="2,7 5.5,10.5 12,3.5" />
                </svg>
              ) : (i + 1)}
            </div>
            <span className={`text-xs mt-1 whitespace-nowrap ${
              i === current ? 'text-zinc-900 dark:text-white font-medium' : 'text-zinc-400 dark:text-zinc-600'
            }`}>{step}</span>
          </div>
          {i < steps.length - 1 && (
            <div className={`h-px w-8 mx-1 mb-5 transition-colors ${
              i < current ? 'bg-zinc-900 dark:bg-white' : 'bg-zinc-200 dark:bg-zinc-700'
            }`} />
          )}
        </div>
      ))}
    </div>
  )
}

// ── Shared UI ─────────────────────────────────────────────────────────────────

function Field({ label, value, onChange, placeholder, textarea }: {
  label: string; value: string; onChange: (v: string) => void
  placeholder?: string; textarea?: boolean
}) {
  const cls = "w-full bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 placeholder-zinc-400 dark:placeholder-zinc-500 rounded-xl px-4 py-3 text-sm focus:outline-none focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600"
  return (
    <div>
      <label className="block text-sm font-medium text-zinc-700 dark:text-zinc-300 mb-1.5">{label}</label>
      {textarea
        ? <textarea value={value} onChange={e => onChange(e.target.value)} placeholder={placeholder} rows={3} className={`${cls} resize-none`} />
        : <input value={value} onChange={e => onChange(e.target.value)} placeholder={placeholder} className={cls} />
      }
    </div>
  )
}

function Select({ label, value, onChange, options, placeholder }: {
  label: string; value: string; onChange: (v: string) => void
  options: { value: string; label: string }[]
  placeholder: string
}) {
  return (
    <div>
      <label className="block text-sm font-medium text-zinc-700 dark:text-zinc-300 mb-1.5">{label}</label>
      <select value={value} onChange={e => onChange(e.target.value)}
        className="w-full bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 rounded-xl px-4 py-3 text-sm focus:outline-none focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600">
        <option value="">{placeholder}</option>
        {options.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
      </select>
    </div>
  )
}

function NavButtons({ onBack, onNext, nextLabel, nextDisabled, loading, backLabel, loadingLabel }: {
  onBack?: () => void; onNext: () => void
  nextLabel: string; nextDisabled?: boolean; loading?: boolean; backLabel: string; loadingLabel: string
}) {
  return (
    <div className="flex justify-between mt-8">
      {onBack
        ? <button onClick={onBack} className="px-5 py-2.5 text-sm text-zinc-500 hover:text-zinc-900 dark:hover:text-white transition-colors">{backLabel}</button>
        : <div />
      }
      <button onClick={onNext} disabled={nextDisabled || loading}
        className="px-6 py-2.5 bg-zinc-900 dark:bg-white text-white dark:text-zinc-900 rounded-xl text-sm font-medium hover:bg-zinc-700 dark:hover:bg-zinc-200 disabled:opacity-40 disabled:cursor-not-allowed transition-colors">
        {loading ? loadingLabel : nextLabel}
      </button>
    </div>
  )
}

function ErrorBox({ message }: { message: string }) {
  return (
    <div className="mt-4 p-3 bg-red-50 dark:bg-red-950/50 border border-red-200 dark:border-red-800 rounded-xl text-red-600 dark:text-red-400 text-sm">
      {message}
    </div>
  )
}

// ── Step 1: Product ───────────────────────────────────────────────────────────

function Step1({ onNext }: { onNext: () => void }) {
  const { t } = useTranslations()
  const { product, setProduct } = usePipelineStore()
  const valid = product.name && product.type && product.audience && product.brandStyle

  return (
    <div>
      <h2 className="text-xl font-semibold text-zinc-900 dark:text-white mb-1">{t.pipeline.step1.title}</h2>
      <p className="text-sm text-zinc-500 mb-6">{t.pipeline.step1.subtitle}</p>
      <div className="space-y-4">
        <Field label={t.pipeline.step1.productName} value={product.name} onChange={v => setProduct({ name: v })} placeholder={t.pipeline.step1.productNamePlaceholder} />
        <Select label={t.pipeline.step1.productType} value={product.type} onChange={v => setProduct({ type: v })} options={t.pipeline.step1.productTypes} placeholder={t.pipeline.step1.selectPlaceholder} />
        <Field label={t.pipeline.step1.audience} value={product.audience} onChange={v => setProduct({ audience: v })} placeholder={t.pipeline.step1.audiencePlaceholder} />
        <Select label={t.pipeline.step1.brandStyle} value={product.brandStyle} onChange={v => setProduct({ brandStyle: v })} options={t.pipeline.step1.brandStyles} placeholder={t.pipeline.step1.selectPlaceholder} />
        <Field label={t.pipeline.step1.description} value={product.description} onChange={v => setProduct({ description: v })}
          placeholder={t.pipeline.step1.descriptionPlaceholder} textarea />
      </div>
      <NavButtons onNext={onNext} nextDisabled={!valid} nextLabel={t.pipeline.step1.nextBtn} backLabel={t.pipeline.back} loadingLabel={t.pipeline.generating} />
    </div>
  )
}

// ── Step 2: Visuals ───────────────────────────────────────────────────────────

function Step2({ onBack, onNext }: { onBack: () => void; onNext: () => void }) {
  const { t } = useTranslations()
  const { product, visuals, setVisuals } = usePipelineStore()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  async function handleGenerate() {
    setLoading(true)
    setError('')
    try {
      const prompt = visuals.imagePrompt || await generateImagePrompt(product)
      setVisuals({ imagePrompt: prompt })
      const images = await generateImages(prompt)
      setVisuals({ images })
    } catch (e) {
      setError(e instanceof Error ? e.message : t.pipeline.step2.generateError)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div>
      <h2 className="text-xl font-semibold text-zinc-900 dark:text-white mb-1">{t.pipeline.step2.title}</h2>
      <p className="text-sm text-zinc-500 mb-6">{t.pipeline.step2.subtitle}</p>
      <div className="space-y-4">
        <Field label={t.pipeline.step2.promptLabel}
          value={visuals.imagePrompt} onChange={v => setVisuals({ imagePrompt: v })}
          placeholder={t.pipeline.step2.promptPlaceholder} textarea />
        <button onClick={handleGenerate} disabled={loading}
          className="w-full py-3 bg-zinc-100 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 rounded-xl text-sm font-medium hover:bg-zinc-200 dark:hover:bg-zinc-700 disabled:opacity-40 transition-colors">
          {loading ? t.pipeline.step2.generatingBtn : visuals.images.length ? t.pipeline.step2.regenerateBtn : t.pipeline.step2.generateBtn}
        </button>
      </div>

      {error && <ErrorBox message={error} />}

      {visuals.images.length > 0 && (
        <div className="mt-6">
          <p className="text-sm font-medium text-zinc-700 dark:text-zinc-300 mb-3">{t.pipeline.step2.selectLabel}</p>
          <div className="grid grid-cols-2 gap-3">
            {visuals.images.map((img, i) => (
              <button key={i} onClick={() => setVisuals({ selectedImage: img })}
                className={`relative rounded-xl overflow-hidden border-2 transition-colors ${
                  visuals.selectedImage === img
                    ? 'border-zinc-900 dark:border-white'
                    : 'border-zinc-200 dark:border-zinc-700 hover:border-zinc-400'
                }`}>
                <img src={img} alt={`${t.pipeline.generatedImageAlt} ${i + 1}`} className="w-full aspect-[3/4] object-cover" />
                {visuals.selectedImage === img && (
                  <div className="absolute top-2 right-2 w-6 h-6 bg-zinc-900 dark:bg-white rounded-full flex items-center justify-center">
                    <svg width="12" height="12" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="text-white dark:text-zinc-900">
                      <polyline points="2,7 5.5,10.5 12,3.5" />
                    </svg>
                  </div>
                )}
              </button>
            ))}
          </div>
        </div>
      )}

      <NavButtons onBack={onBack} onNext={onNext} nextLabel={t.pipeline.step2.nextBtn}
        nextDisabled={visuals.images.length > 0 && !visuals.selectedImage} backLabel={t.pipeline.back} loadingLabel={t.pipeline.generating} />
    </div>
  )
}

// ── Step 3: SEO ───────────────────────────────────────────────────────────────

function Step3({ onBack, onNext }: { onBack: () => void; onNext: () => void }) {
  const { t } = useTranslations()
  const { product, seo, setSeo } = usePipelineStore()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const generated = seo.h1 || seo.title

  async function handleGenerate() {
    setLoading(true)
    setError('')
    try {
      const result = await generateSeo(product)
      setSeo(result)
    } catch (e) {
      setError(e instanceof Error ? e.message : t.pipeline.step3.generateError)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div>
      <h2 className="text-xl font-semibold text-zinc-900 dark:text-white mb-1">{t.pipeline.step3.title}</h2>
      <p className="text-sm text-zinc-500 mb-6">{t.pipeline.step3.subtitle}</p>
      {!generated && (
        <button onClick={handleGenerate} disabled={loading}
          className="w-full py-3 bg-zinc-100 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 rounded-xl text-sm font-medium hover:bg-zinc-200 dark:hover:bg-zinc-700 disabled:opacity-40 transition-colors mb-4">
          {loading ? t.pipeline.step3.generatingBtn : t.pipeline.step3.generateBtn}
        </button>
      )}
      {error && <ErrorBox message={error} />}
      {generated && (
        <div className="space-y-4">
          <Field label={t.pipeline.step3.h1Label} value={seo.h1} onChange={v => setSeo({ h1: v })} placeholder={t.pipeline.step3.h1Placeholder} />
          <Field label={t.pipeline.step3.titleLabel} value={seo.title} onChange={v => setSeo({ title: v })} placeholder={t.pipeline.step3.titlePlaceholder} />
          <Field label={t.pipeline.step3.metaLabel} value={seo.metaDescription} onChange={v => setSeo({ metaDescription: v })} placeholder={t.pipeline.step3.metaPlaceholder} textarea />
          <Field label={t.pipeline.step3.altLabel} value={seo.altText} onChange={v => setSeo({ altText: v })} placeholder={t.pipeline.step3.altPlaceholder} />
          <button onClick={handleGenerate} disabled={loading}
            className="text-sm text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-300 transition-colors">
            {loading ? t.pipeline.regenerating : t.pipeline.regenerate}
          </button>
        </div>
      )}
      <NavButtons onBack={onBack} onNext={onNext} nextLabel={t.pipeline.step3.nextBtn} nextDisabled={!generated} loading={loading && !generated} backLabel={t.pipeline.back} loadingLabel={t.pipeline.generating} />
    </div>
  )
}

// ── Step 4: Content ───────────────────────────────────────────────────────────

function Step4({ onBack, onNext }: { onBack: () => void; onNext: () => void }) {
  const { t } = useTranslations()
  const { product, content, setContent } = usePipelineStore()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const generated = !!content.heroHeadline

  async function handleGenerate() {
    setLoading(true)
    setError('')
    try {
      const result = await generateContent(product)
      setContent(result)
    } catch (e) {
      setError(e instanceof Error ? e.message : t.pipeline.step4.generateError)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div>
      <h2 className="text-xl font-semibold text-zinc-900 dark:text-white mb-1">{t.pipeline.step4.title}</h2>
      <p className="text-sm text-zinc-500 mb-6">{t.pipeline.step4.subtitle}</p>
      {!generated && (
        <button onClick={handleGenerate} disabled={loading}
          className="w-full py-3 bg-zinc-100 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 rounded-xl text-sm font-medium hover:bg-zinc-200 dark:hover:bg-zinc-700 disabled:opacity-40 transition-colors mb-4">
          {loading ? t.pipeline.step4.generatingBtn : t.pipeline.step4.generateBtn}
        </button>
      )}
      {error && <ErrorBox message={error} />}
      {generated && (
        <div className="space-y-4">
          <Field label={t.pipeline.step4.heroHeadlineLabel} value={content.heroHeadline} onChange={v => setContent({ heroHeadline: v })} placeholder={t.pipeline.step4.heroHeadlinePlaceholder} />
          <Field label={t.pipeline.step4.heroSubtextLabel} value={content.heroSubtext} onChange={v => setContent({ heroSubtext: v })} placeholder={t.pipeline.step4.heroSubtextPlaceholder} textarea />
          <div>
            <label className="block text-sm font-medium text-zinc-700 dark:text-zinc-300 mb-1.5">{t.pipeline.step4.featuresLabel}</label>
            <div className="space-y-2">
              {(content.features.length ? content.features : ['', '', '']).map((f, i) => (
                <input key={i} value={f} onChange={e => {
                  const next = [...(content.features.length ? content.features : ['', '', ''])]
                  next[i] = e.target.value
                  setContent({ features: next })
                }} placeholder={`${t.pipeline.step4.featurePlaceholder} ${i + 1}`}
                  className="w-full bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 placeholder-zinc-400 dark:placeholder-zinc-500 rounded-xl px-4 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600" />
              ))}
            </div>
          </div>
          <Field label={t.pipeline.step4.ctaLabel} value={content.ctaText} onChange={v => setContent({ ctaText: v })} placeholder={t.pipeline.step4.ctaPlaceholder} />
          <Field label={t.pipeline.step4.testimonialLabel} value={content.testimonial} onChange={v => setContent({ testimonial: v })} placeholder={t.pipeline.step4.testimonialPlaceholder} textarea />
          <button onClick={handleGenerate} disabled={loading}
            className="text-sm text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-300 transition-colors">
            {loading ? t.pipeline.regenerating : t.pipeline.regenerate}
          </button>
        </div>
      )}
      <NavButtons onBack={onBack} onNext={onNext} nextLabel={t.pipeline.step4.nextBtn} nextDisabled={!generated} loading={loading && !generated} backLabel={t.pipeline.back} loadingLabel={t.pipeline.generating} />
    </div>
  )
}

// ── Step 5: Styles ────────────────────────────────────────────────────────────

function Step5({ onBack, onNext }: { onBack: () => void; onNext: () => void }) {
  const { t } = useTranslations()
  const { product, styles, setStyles } = usePipelineStore()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [reasoning, setReasoning] = useState('')

  async function handleGenerate() {
    setLoading(true)
    setError('')
    try {
      const result = await generateStyles(product)
      setStyles(result)
      setReasoning(result.reasoning)
    } catch (e) {
      setError(e instanceof Error ? e.message : t.pipeline.step5.generateError)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div>
      <h2 className="text-xl font-semibold text-zinc-900 dark:text-white mb-1">{t.pipeline.step5.title}</h2>
      <p className="text-sm text-zinc-500 mb-6">{t.pipeline.step5.subtitle}</p>
      <div className="space-y-5">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-zinc-700 dark:text-zinc-300 mb-1.5">{t.pipeline.step5.primaryColor}</label>
            <div className="flex items-center gap-3">
              <input type="color" value={styles.primaryColor} onChange={e => setStyles({ primaryColor: e.target.value })}
                className="w-10 h-10 rounded-lg cursor-pointer border-0 bg-transparent" />
              <span className="text-sm text-zinc-500 font-mono">{styles.primaryColor}</span>
            </div>
          </div>
          <div>
            <label className="block text-sm font-medium text-zinc-700 dark:text-zinc-300 mb-1.5">{t.pipeline.step5.secondaryColor}</label>
            <div className="flex items-center gap-3">
              <input type="color" value={styles.secondaryColor} onChange={e => setStyles({ secondaryColor: e.target.value })}
                className="w-10 h-10 rounded-lg cursor-pointer border-0 bg-transparent" />
              <span className="text-sm text-zinc-500 font-mono">{styles.secondaryColor}</span>
            </div>
          </div>
        </div>
        <Select label={t.pipeline.step5.fontStyle} value={styles.fontStyle} onChange={v => setStyles({ fontStyle: v })} options={t.pipeline.step5.fontStyles} placeholder={t.pipeline.step1.selectPlaceholder} />
        <Select label={t.pipeline.step5.animationStyle} value={styles.animationStyle} onChange={v => setStyles({ animationStyle: v })} options={t.pipeline.step5.animationStyles} placeholder={t.pipeline.step1.selectPlaceholder} />
        <button onClick={handleGenerate} disabled={loading}
          className="w-full py-3 bg-zinc-100 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 rounded-xl text-sm font-medium hover:bg-zinc-200 dark:hover:bg-zinc-700 disabled:opacity-40 transition-colors">
          {loading ? t.pipeline.generating : t.pipeline.step5.aiPickBtn}
        </button>
        {reasoning && (
          <p className="text-xs text-zinc-400 dark:text-zinc-500 italic">{reasoning}</p>
        )}
      </div>
      {error && <ErrorBox message={error} />}
      <NavButtons onBack={onBack} onNext={onNext} nextLabel={t.pipeline.step5.nextBtn} backLabel={t.pipeline.back} loadingLabel={t.pipeline.generating} />
    </div>
  )
}

// ── Step 6: Assemble ──────────────────────────────────────────────────────────

function Step6({ onBack }: { onBack: () => void }) {
  const { t } = useTranslations()
  const { product, visuals, seo, content, styles, finalHtml, setFinalHtml, reset } = usePipelineStore()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  async function handleAssemble() {
    setLoading(true)
    setError('')
    try {
      const html = await assemblePage(product, visuals.selectedImage, seo, content, styles)
      setFinalHtml(html)
    } catch (e) {
      setError(e instanceof Error ? e.message : t.pipeline.step6.assembleError)
    } finally {
      setLoading(false)
    }
  }

  function handleDownload() {
    const blob = new Blob([finalHtml], { type: 'text/html' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${product.name.toLowerCase().replace(/\s+/g, '-')}-landing.html`
    a.click()
  }

  return (
    <div>
      <h2 className="text-xl font-semibold text-zinc-900 dark:text-white mb-1">{t.pipeline.step6.title}</h2>
      <p className="text-sm text-zinc-500 mb-6">{t.pipeline.step6.subtitle}</p>

      {!finalHtml && (
        <button onClick={handleAssemble} disabled={loading}
          className="w-full py-4 bg-zinc-900 dark:bg-white text-white dark:text-zinc-900 rounded-xl text-sm font-semibold hover:bg-zinc-700 dark:hover:bg-zinc-200 disabled:opacity-40 transition-colors">
          {loading ? t.pipeline.step6.assemblingBtn : t.pipeline.step6.assembleBtn}
        </button>
      )}

      {loading && (
        <div className="mt-8 flex flex-col items-center gap-3">
          <div className="w-8 h-8 border-2 border-zinc-300 dark:border-zinc-600 border-t-zinc-900 dark:border-t-white rounded-full animate-spin" />
          <p className="text-sm text-zinc-500">{t.pipeline.step6.assembling}</p>
        </div>
      )}

      {error && <ErrorBox message={error} />}

      {finalHtml && (
        <div className="space-y-4">
          <div className="flex gap-3">
            <button onClick={handleDownload}
              className="flex-1 py-2.5 bg-zinc-900 dark:bg-white text-white dark:text-zinc-900 rounded-xl text-sm font-medium hover:bg-zinc-700 dark:hover:bg-zinc-200 transition-colors">
              {t.pipeline.step6.download}
            </button>
            <button onClick={() => { setFinalHtml(''); handleAssemble() }} disabled={loading}
              className="px-4 py-2.5 bg-zinc-100 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 rounded-xl text-sm font-medium hover:bg-zinc-200 dark:hover:bg-zinc-700 disabled:opacity-40 transition-colors">
              {t.pipeline.step6.regenerate}
            </button>
          </div>
          <iframe srcDoc={finalHtml} className="w-full rounded-xl border border-zinc-200 dark:border-zinc-700 bg-white"
            style={{ height: '70vh' }} title={t.pipeline.generatedPageTitle} sandbox="allow-scripts" />
          <button onClick={reset} className="text-sm text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-300 transition-colors">
            {t.pipeline.step6.startOver}
          </button>
        </div>
      )}

      {!finalHtml && <NavButtons onBack={onBack} onNext={handleAssemble} nextLabel={t.pipeline.assemble} loading={loading} backLabel={t.pipeline.back} loadingLabel={t.pipeline.generating} />}
    </div>
  )
}

// ── Main page ─────────────────────────────────────────────────────────────────

export default function AdvancedPipelinePage() {
  const { t } = useTranslations()
  const { currentStep, setStep } = usePipelineStore()
  const steps = t.pipeline.steps

  function next() { setStep(Math.min(currentStep + 1, steps.length - 1)) }
  function back() { setStep(Math.max(currentStep - 1, 0)) }

  return (
    <div className="min-h-screen p-8">
      <div className="max-w-2xl mx-auto">
        <div className="mb-6">
          <h1 className="text-2xl font-bold text-zinc-900 dark:text-white">{t.pipeline.title}</h1>
          <p className="text-sm text-zinc-500 mt-1">{t.pipeline.subtitle}</p>
        </div>
        <StepIndicator current={currentStep} steps={steps} />
        <div className="bg-zinc-50 dark:bg-zinc-900 rounded-2xl border border-zinc-200 dark:border-zinc-800 p-6">
          {currentStep === 0 && <Step1 onNext={next} />}
          {currentStep === 1 && <Step2 onBack={back} onNext={next} />}
          {currentStep === 2 && <Step3 onBack={back} onNext={next} />}
          {currentStep === 3 && <Step4 onBack={back} onNext={next} />}
          {currentStep === 4 && <Step5 onBack={back} onNext={next} />}
          {currentStep === 5 && <Step6 onBack={back} />}
        </div>
      </div>
    </div>
  )
}
