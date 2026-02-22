import { PanelCard } from '@/components/primitives/PanelCard'
import { PanelCheckbox } from '@/components/primitives/PanelCheckbox'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'
import type { PanelSection } from './parsePanel'

type PanelSectionRendererProps = {
  section: PanelSection
  selections: Map<string, boolean>
  onToggle: (id: string) => void
}

function PanelSectionRenderer({ section, selections, onToggle }: PanelSectionRendererProps) {
  return (
    <PanelCard depth={section.depth} title={section.title}>
      {section.bodyMarkdown ? <TerminalBlock content={section.bodyMarkdown} /> : null}

      {section.interactiveItems.map((item) => (
        <PanelCheckbox
          key={item.id}
          label={item.label}
          checked={selections.get(item.id) ?? item.checked}
          onChange={() => onToggle(item.id)}
        />
      ))}

      {section.children.map((child) => (
        <PanelSectionRenderer
          key={child.id}
          section={child}
          selections={selections}
          onToggle={onToggle}
        />
      ))}
    </PanelCard>
  )
}

export { PanelSectionRenderer }
