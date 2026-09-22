import type { Metadata } from 'rari'
import { PageTransition } from '../page-transition'
import { BrowserOnlyDemo } from './browser-only-demo'
import { DemoUserContext, DemoUserLabel } from './demo-user-context'
import { ViewTransitionImageDemo } from './view-transition-image-demo'

export default function React19Page() {
  return (
    <PageTransition>
      <div className="space-y-8">
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 md:p-12">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">React 19.3</h1>
          <p className="text-lg text-gray-600 leading-relaxed max-w-3xl">
            Smoke demos for View Transitions (including route navigations), Fragment-friendly
            patterns, <code>use(browser())</code>, Context rendered from Server Components, and
            Image waiting inside <code>&lt;ViewTransition&gt;</code>.
          </p>
          <p className="text-sm text-gray-500 mt-4">
            Soft navigations run inside <code>startTransition</code> with{' '}
            <code>addTransitionType</code>. Two patterns in <code>page-transition.tsx</code> (on the
            page / <code>loading.tsx</code>, not template): type-keyed route enter/exit, and
            string-prop Suspense reveal — so loading is never a shared morph target.
          </p>
        </div>

        <section className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 space-y-4">
          <h2 className="text-2xl font-bold text-gray-900">Context from a Server Component</h2>
          <p className="text-sm text-gray-600">
            The server page renders <code>&lt;DemoUserContext value=&#123;...&#125;&gt;</code>{' '}
            directly — no client Provider wrapper.
          </p>
          <DemoUserContext value={{ name: 'Ada', role: 'admin' }}>
            <DemoUserLabel />
          </DemoUserContext>
        </section>

        <section className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 space-y-4">
          <h2 className="text-2xl font-bold text-gray-900">browser()</h2>
          <BrowserOnlyDemo />
        </section>

        <section className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 space-y-4">
          <h2 className="text-2xl font-bold text-gray-900">View Transition + Image</h2>
          <p className="text-sm text-gray-600">
            Image omits <code>onLoad</code> unless needed, so React can wait for the image during
            the transition.
          </p>
          <ViewTransitionImageDemo />
        </section>
      </div>
    </PageTransition>
  )
}

export const metadata: Metadata = {
  title: 'React 19.3 Demos',
  description: 'View Transitions, browser(), and Context-from-RSC examples',
}
