import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { rscClient } from 'virtual:rsc-integration'
import App from './App.tsx'

import './index.css'
import 'virtual:rsc-client-components'

try {
  rscClient.configure({ enableStreaming: false })
}
catch {}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
