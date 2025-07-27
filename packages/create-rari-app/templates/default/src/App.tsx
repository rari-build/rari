import { RouterProvider } from 'rari/client'
import { routes } from './.rari/routes'

function App() {
  return (
    <RouterProvider
      routes={routes}
      config={{
        basePath: '',
        useHash: false,
        caseSensitive: false,
      }}
    >
      <div id="app" />
    </RouterProvider>
  )
}

export default App
