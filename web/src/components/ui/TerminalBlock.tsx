import { code } from '@/lib/site/styles'
import Terminal2 from '../icons/Terminal2'
import CommandPanel from './CommandPanel'

interface TerminalBlockProps {
  readonly command: string
  readonly showHeader?: boolean
}

export default function TerminalBlock({ command, showHeader = true }: TerminalBlockProps) {
  return (
    <CommandPanel
      command={command}
      header={
        showHeader ? (
          <div className={code.header}>
            <Terminal2 className="w-4 h-4 text-fg-muted shrink-0" />
            <span className="text-sm text-fg-muted font-medium">Terminal</span>
          </div>
        ) : undefined
      }
    />
  )
}
