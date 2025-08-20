import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'

import 'virtual:rsc-integration'
import 'virtual:rsc-client-components'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
