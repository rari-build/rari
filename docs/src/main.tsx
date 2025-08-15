import React from 'react'
import ReactDOM from 'react-dom/client'
import { rscClient } from 'virtual:rsc-integration'
import App from './App.tsx'

import './index.css'
import 'virtual:rsc-client-components'

try {
  rscClient.configure({ enableStreaming: false })
}
catch {}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
