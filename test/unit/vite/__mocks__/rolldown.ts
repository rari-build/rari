import { vi } from 'vite-plus/test'

export const build = vi.fn().mockResolvedValue({
  output: [{
    type: 'chunk',
    isEntry: true,
    code: 'export default function Component() { return "mocked" }',
  }],
})
