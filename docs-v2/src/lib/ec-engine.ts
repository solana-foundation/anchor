import { ExpressiveCodeEngine, ExpressiveCodeTheme } from '@expressive-code/core'
import { ecOptions, ecDefaultPlugins, EC_THEME_NAMES } from './ec-config'

let enginePromise: Promise<ExpressiveCodeEngine> | null = null
let stylesPromise: Promise<string> | null = null

async function loadTheme(name: string): Promise<ExpressiveCodeTheme> {
  const { bundledThemes } = await import('shiki')
  const loader = bundledThemes[name as keyof typeof bundledThemes]
  if (!loader) throw new Error(`Unknown shiki theme: ${name}`)
  const themeData = await loader()
  return new ExpressiveCodeTheme(themeData.default as any)
}

export function getEcEngine(): Promise<ExpressiveCodeEngine> {
  if (!enginePromise) {
    enginePromise = (async () => {
      const themes = await Promise.all(EC_THEME_NAMES.map(loadTheme))
      return new ExpressiveCodeEngine({
        themes,
        ...ecOptions,
        plugins: [...ecDefaultPlugins(), ...ecOptions.plugins],
      })
    })()
  }
  return enginePromise
}

export function getEcStyles(): Promise<string> {
  if (!stylesPromise) {
    stylesPromise = (async () => {
      const engine = await getEcEngine()
      const base = await engine.getBaseStyles()
      const themes = await engine.getThemeStyles()
      return base + themes
    })()
  }
  return stylesPromise
}
