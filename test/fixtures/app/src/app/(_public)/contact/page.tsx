import { Callout } from '@/app/_components/Callout'

export default function ContactPage() {
  return (
    <div>
      <h1>Contact</h1>
      <div data-testid="contact-content">Reach us at contact@example.com</div>
      <Callout>
        <span data-testid="private-callout-content">Imported from _components</span>
      </Callout>
    </div>
  )
}
