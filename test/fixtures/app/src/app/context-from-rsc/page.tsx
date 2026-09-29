import { DemoUserContext, DemoUserLabel } from './demo-user-context'

export default function ContextFromRscPage() {
  return (
    <div>
      <h1 data-testid="page-title">Context from RSC</h1>
      <DemoUserContext value={{ name: 'Ada', role: 'admin' }}>
        <DemoUserLabel />
      </DemoUserContext>
    </div>
  )
}
