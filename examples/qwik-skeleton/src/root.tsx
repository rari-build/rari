import { component$ } from '@qwik.dev/core'
import { DocumentHeadTags, RouterOutlet, useQwikRouter } from '@qwik.dev/router'

// Document root, exactly as in a standalone Qwik Router app. rari hosts the
// request; Qwik Router owns routing, loaders, actions and the document head.
export default component$(() => {
  useQwikRouter()
  return (
    <>
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <DocumentHeadTags />
      </head>
      <body>
        <RouterOutlet />
      </body>
    </>
  )
})
