import type { Metadata } from 'rari'
import type { ComponentType, SVGProps } from 'react'
import ArrowNarrowRight from '@/components/icons/ArrowNarrowRight'
import Check from '@/components/icons/Check'
import Cloudflare from '@/components/icons/sponsors/Cloudflare'
import Neon from '@/components/icons/sponsors/Neon'
import Sanity from '@/components/icons/sponsors/Sanity'
import Sentry from '@/components/icons/sponsors/Sentry'

const SPONSOR_URL = 'https://github.com/sponsors/skiniks'

function hexWithAlpha(hex: string, alpha: string): string {
  if (/^#[0-9A-F]{6}$/i.test(hex)) {
    return `${hex}${alpha}`
  }

  const normalizedHex = hex.replace('#', '')
  let r: number, g: number, b: number

  if (normalizedHex.length === 3) {
    r = Number.parseInt(normalizedHex[0] + normalizedHex[0], 16)
    g = Number.parseInt(normalizedHex[1] + normalizedHex[1], 16)
    b = Number.parseInt(normalizedHex[2] + normalizedHex[2], 16)
  }
  else if (normalizedHex.length === 6) {
    r = Number.parseInt(normalizedHex.substring(0, 2), 16)
    g = Number.parseInt(normalizedHex.substring(2, 4), 16)
    b = Number.parseInt(normalizedHex.substring(4, 6), 16)
  }
  else {
    r = 0
    g = 0
    b = 0
  }

  const numericAlpha = Number.parseInt(alpha, 16) / 255

  return `rgba(${r}, ${g}, ${b}, ${numericAlpha})`
}

const neonPartner = {
  href: 'https://get.neon.com/KDQudHN',
  label: 'Neon - Serverless Postgres',
  Icon: Neon,
  color: '#34D59A',
  description: 'Serverless Postgres hosting',
}

interface Benefit {
  text: string
  isStatement?: boolean
}

interface Tier {
  name: string
  price: number
  description: string
  benefits: Benefit[]
  highlight?: boolean
}

const recurringTiers: Tier[] = [
  {
    name: 'Supporter',
    price: 5,
    description: 'Support rari\'s development. Every contribution helps keep the framework fast, open, and free for everyone.',
    benefits: [
      { text: 'Supporter badge in Discord' },
      { text: 'Monthly progress updates via email' },
      { text: 'This tier is purely to support the project if you believe in it!' },
    ],
  },
  {
    name: 'Silver',
    price: 25,
    description: 'For developers and teams evaluating rari. Get visibility for your support and help shape the roadmap.',
    benefits: [
      { text: 'Everything in Supporter tier' },
      { text: 'Your name on rari.build/sponsors (Silver tier)' },
      { text: 'Vote on roadmap priorities (quarterly polls)' },
      { text: 'Priority response to bug reports (48hr SLA)' },
    ],
  },
  {
    name: 'Gold',
    price: 100,
    description: 'Running rari in production or planning to? Get direct support to ensure your deployments stay fast and stable.',
    benefits: [
      { text: 'Everything in Silver tier' },
      { text: 'Your logo on rari.build/sponsors (Gold tier)' },
      { text: 'Direct Discord access to me' },
      { text: 'Priority production issue support (24hr SLA)' },
      { text: 'Monthly office hours slot (30 min, by appointment)' },
    ],
    highlight: true,
  },
  {
    name: 'Platinum',
    price: 500,
    description: 'For companies betting on rari. Get dedicated support, roadmap influence, and co-marketing opportunities.',
    benefits: [
      { text: 'Everything in Gold tier' },
      { text: 'Premium logo placement (Platinum tier - top of page + README)' },
      { text: 'Extended office hours (60 min/month instead of 30)' },
      { text: 'Migration consulting (2 hours/month included)' },
      { text: 'Your feature requests jump the queue' },
      { text: 'Optional case study feature on rari.build' },
    ],
  },
  {
    name: 'Diamond',
    price: 2500,
    description: 'For companies that need dedicated partnership, custom development, or want to significantly accelerate rari\'s growth.',
    benefits: [
      { text: 'Everything in Platinum tier' },
      { text: 'Diamond tier placement (largest logos, top billing)' },
      { text: 'Dedicated Slack/Discord channel for your team' },
      { text: 'Weekly check-ins available' },
      { text: 'Custom feature development (aligned with roadmap)' },
      { text: 'Official "Technology Partner" designation' },
      { text: 'Joint conference speaking opportunities' },
      { text: 'First access to any commercial offerings' },
      { text: 'Let\'s build something together.', isStatement: true },
    ],
  },
]

const oneTimeTiers: Tier[] = [
  {
    name: 'Quick Thanks',
    price: 50,
    description: 'Quick way to say thanks.',
    benefits: [
      { text: 'Supporter badge in Discord' },
      { text: 'Shout out in the monthly update email' },
    ],
  },
  {
    name: 'Kickstart',
    price: 100,
    description: 'Kickstart development!',
    benefits: [
      { text: 'Same benefits as Silver tier for one month' },
      { text: 'Your name on rari.build/sponsors' },
      { text: 'Priority support for 30 days' },
    ],
  },
]

interface InfrastructurePartner {
  href: string
  label: string
  Icon: ComponentType<SVGProps<SVGSVGElement>>
  color: string
  secondaryColor?: string
  description: string
}

const infrastructurePartners: InfrastructurePartner[] = [
  neonPartner,
  {
    href: 'https://cloudflare.com',
    label: 'Cloudflare - CDN & Infrastructure',
    Icon: Cloudflare,
    color: '#f48120',
    secondaryColor: '#faad3f',
    description: 'CDN, R2 storage & DDoS protection',
  },
  {
    href: 'https://sanity.io',
    label: 'Sanity - Content Platform',
    Icon: Sanity,
    color: '#F04939',
    secondaryColor: '#F37368',
    description: 'Content platform & CMS',
  },
  {
    href: 'https://sentry.io',
    label: 'Sentry - Error Monitoring',
    Icon: Sentry,
    color: '#362d59',
    description: 'Error monitoring & observability',
  },
]

function TierCard({ tier }: { tier: Tier }) {
  return (
    <a
      href={SPONSOR_URL}
      target="_blank"
      rel="noopener noreferrer"
      className={`relative group h-full overflow-hidden rounded-xl p-px block ${tier.highlight ? 'md:scale-105' : ''}`}
    >
      {tier.highlight && (
        <div className="absolute -inset-0.5 bg-linear-to-r from-[#fd7e14] to-[#e8590c] rounded-xl blur opacity-30 group-hover:opacity-40 transition-opacity"></div>
      )}
      <div className={`relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border ${tier.highlight ? 'border-[#fd7e14]' : 'border-[#30363d]'} rounded-xl p-6 flex flex-col transition-all duration-300 group-hover:border-transparent`}>
        <div className="absolute inset-0 bg-linear-to-br from-[#fd7e14]/10 via-[#e8590c]/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl"></div>
        <div className="relative z-10 flex flex-col h-full">
          <div className="mb-4">
            <h3 className="text-2xl font-bold text-[#f0f6fc] mb-2">{tier.name}</h3>
            <div className="flex items-baseline gap-1 mb-3">
              <span className="text-4xl font-bold text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">
                $
                {tier.price.toLocaleString()}
              </span>
              <span className="text-gray-400">/month</span>
            </div>
            <p className="text-gray-400 text-sm leading-relaxed group-hover:text-gray-300 transition-colors duration-300">{tier.description}</p>
          </div>

          <div className="flex-1">
            <div className="space-y-3">
              {tier.benefits.map((benefit, i) => (
                <div key={i} className={`flex items-start gap-2 ${benefit.isStatement ? 'mt-4 pt-4 border-t border-[#30363d]' : ''}`}>
                  {!benefit.isStatement && (
                    <Check className="w-5 h-5 text-[#fd7e14] shrink-0 mt-0.5" />
                  )}
                  <span className={`text-gray-300 text-sm ${benefit.isStatement ? 'italic text-center w-full' : ''}`}>{benefit.text}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </a>
  )
}

function OneTimeTierCard({ tier }: { tier: Tier }) {
  return (
    <a
      href={SPONSOR_URL}
      target="_blank"
      rel="noopener noreferrer"
      className="relative group h-full overflow-hidden rounded-xl p-px block"
    >
      <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
        <div className="absolute inset-0 bg-linear-to-br from-[#fd7e14]/10 via-[#e8590c]/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl"></div>
        <div className="relative z-10 flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
          <div className="flex-1">
            <h3 className="text-xl font-bold text-[#f0f6fc] mb-2">{tier.name}</h3>
            <p className="text-gray-400 text-sm mb-3 group-hover:text-gray-300 transition-colors duration-300">{tier.description}</p>
            <div className="space-y-2">
              {tier.benefits.map((benefit, i) => (
                <div key={i} className="flex items-start gap-2">
                  <Check className="w-4 h-4 text-[#fd7e14] shrink-0 mt-0.5" />
                  <span className="text-gray-300 text-sm">{benefit.text}</span>
                </div>
              ))}
            </div>
          </div>
          <div className="shrink-0">
            <span className="text-3xl font-bold text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">
              $
              {tier.price.toLocaleString()}
            </span>
          </div>
        </div>
      </div>
    </a>
  )
}

export default function SponsorsPage() {
  return (
    <div className="min-h-screen bg-[#0d1117] text-white">
      <div className="relative overflow-hidden w-full flex items-center">
        <div className="absolute inset-0 bg-linear-to-b from-[#161b22]/30 via-transparent to-transparent"></div>
        <div className="absolute bottom-0 left-0 right-0 h-40 bg-linear-to-t from-[#0d1117] to-transparent pointer-events-none"></div>

        <div className="relative max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-20 w-full">
          <div className="text-center">
            <h1 className="text-4xl lg:text-6xl font-bold text-white mb-6 max-w-3xl mx-auto leading-tight">
              Partner with
              {' '}
              <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">
                rari
              </span>
            </h1>

            <p className="text-lg lg:text-xl text-gray-400 mb-12 max-w-3xl mx-auto leading-relaxed text-balance">
              Support rari's development while getting the tools and support your team needs.
              From individual developers to enterprise teams, we have a tier that fits.
            </p>

            <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
              <a
                href={SPONSOR_URL}
                target="_blank"
                rel="noopener noreferrer"
                className="group relative w-full sm:w-auto px-8 py-4 bg-linear-to-r from-[#fd7e14] to-[#e8590c] text-gray-900 rounded-lg font-semibold text-lg transition-transform duration-200 hover:scale-105 flex items-center justify-center gap-2"
              >
                Become a sponsor
              </a>

              <a
                href="mailto:enterprise@rari.build"
                className="group w-full sm:w-auto px-8 py-4 border-2 border-[#30363d] text-gray-300 hover:text-white hover:border-[#fd7e14] rounded-lg font-semibold text-lg transition-all duration-200 hover:bg-[#161b22]/50 backdrop-blur-sm inline-flex items-center justify-center gap-2"
              >
                Custom partnership
                <ArrowNarrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
              </a>
            </div>
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl lg:text-5xl font-bold text-[#f0f6fc] mb-4">
              Monthly
              {' '}
              <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">sponsorship</span>
            </h2>
            <p className="text-xl text-gray-400 max-w-2xl mx-auto">
              Ongoing support with increasing benefits at every tier
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
            {recurringTiers.slice(0, 3).map(tier => (
              <TierCard key={tier.name} tier={tier} />
            ))}
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            {recurringTiers.slice(3).map(tier => (
              <TierCard key={tier.name} tier={tier} />
            ))}
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl lg:text-5xl font-bold text-[#f0f6fc] mb-4">
              One-time
              {' '}
              <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">contributions</span>
            </h2>
            <p className="text-xl text-gray-400 max-w-2xl mx-auto">
              A quick way to show your support
            </p>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            {oneTimeTiers.map(tier => (
              <OneTimeTierCard key={tier.name} tier={tier} />
            ))}
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl lg:text-5xl font-bold text-[#f0f6fc] mb-4">
              Infrastructure
              {' '}
              <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">partners</span>
            </h2>
            <p className="text-xl text-gray-400 max-w-2xl mx-auto text-balance">
              These companies provide infrastructure and services that power rari's development
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
            {infrastructurePartners.map((partner) => {
              const { href, label, Icon, color, secondaryColor, description } = partner
              return (
                <a
                  key={href}
                  href={href}
                  target="_blank"
                  rel="noopener noreferrer"
                  aria-label={label}
                  className="relative group h-full overflow-hidden rounded-xl p-px block"
                >
                  <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-8 transition-all duration-300 group-hover:border-transparent">
                    <div
                      className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl"
                      style={{
                        background: secondaryColor
                          ? `linear-gradient(to bottom right, ${hexWithAlpha(color, '1a')}, ${hexWithAlpha(secondaryColor, '0d')}, transparent)`
                          : `linear-gradient(to bottom right, ${hexWithAlpha(color, '1a')}, ${hexWithAlpha(color, '0d')}, transparent)`,
                      }}
                    >
                    </div>
                    <div className="relative z-10 flex flex-col items-center text-center gap-4">
                      <div className="transform group-hover:scale-105 transition-transform duration-300">
                        <Icon className="h-10 w-auto" />
                      </div>
                      <p className="text-sm text-gray-400 group-hover:text-gray-300 transition-colors duration-300">{description}</p>
                    </div>
                  </div>
                </a>
              )
            })}
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl lg:text-5xl font-bold text-[#f0f6fc] mb-4">
              Financial
              {' '}
              <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">sponsors</span>
            </h2>
          </div>

          <div className="space-y-8">
            <div>
              <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4 text-center">Launch Partner</h3>
              <div className="relative bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-8">
                <div className="flex items-center justify-center">
                  <a
                    href={neonPartner.href}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="group relative"
                    aria-label={neonPartner.label}
                  >
                    <neonPartner.Icon className="relative h-10 w-auto transition-transform duration-300 group-hover:scale-105" />
                  </a>
                </div>
              </div>
            </div>

            <div className="relative">
              <div className="absolute -inset-0.5 bg-linear-to-r from-[#fd7e14] to-[#e8590c] rounded-2xl blur opacity-20"></div>
              <div className="relative bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-2xl p-8 lg:p-12 text-center">
                <p className="text-xl text-gray-400 mb-8">
                  Join Neon in supporting rari!
                </p>
                <a
                  href={SPONSOR_URL}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="group relative px-8 py-4 bg-linear-to-r from-[#fd7e14] to-[#e8590c] text-gray-900 rounded-lg font-semibold text-lg transition-transform duration-200 hover:scale-105 inline-flex items-center justify-center gap-2"
                >
                  Become a sponsor
                  <ArrowNarrowRight className="w-5 h-5" />
                </a>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Sponsorship Tiers / rari Enterprise',
  description: 'Partner with rari. Choose from monthly sponsorship tiers or make a one-time contribution. Get priority support, roadmap influence, and dedicated partnership opportunities.',
}
