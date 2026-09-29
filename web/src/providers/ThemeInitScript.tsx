import { THEME_STORAGE_KEY } from './theme-constants'

const THEME_INIT_SCRIPT = `(function(){try{var stored=localStorage.getItem(${JSON.stringify(THEME_STORAGE_KEY)});var preference=stored==='light'||stored==='dark'||stored==='system'?stored:'system';var resolved=preference==='system'?window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light':preference;document.documentElement.classList.toggle('light',resolved==='light');document.documentElement.classList.toggle('dark',resolved==='dark')}catch(e){document.documentElement.classList.add('dark')}})()`

export function ThemeInitScript() {
  return (
    <script
      // eslint-disable-next-line react/dom-no-dangerously-set-innerhtml
      dangerouslySetInnerHTML={{ __html: THEME_INIT_SCRIPT }}
    />
  )
}
