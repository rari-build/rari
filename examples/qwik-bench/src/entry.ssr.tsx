import { createRenderer } from '@qwik.dev/router'
import Root from './root'

// The standard Qwik SSR entry. @rari/qwik bundles it with Qwik Router's
// requestHandler and the rari platform into the server bundle rari loads.
export default createRenderer(opts => ({
  jsx: <Root />,
  options: {
    ...opts,
    containerAttributes: { lang: 'en', ...opts.containerAttributes },
  },
}))
