import { memo, useState, useCallback } from 'react'
import type { NodeProps } from '@xyflow/react'
import EditOutlined from '@mui/icons-material/EditOutlined'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import InputOutlined from '@mui/icons-material/InputOutlined'
import SettingsOutlined from '@mui/icons-material/SettingsOutlined'
import { CanvasFormNode } from '../CanvasFormNode'
import type { CanvasFormTab } from '../CanvasFormNode'
import { PROTOCOL_TYPE_COLORS } from '../constants'
import { DocumenterHeader } from './DocumenterHeader'
import { PromptTab } from './tabs/PromptTab'
import { DocumentsTab } from './tabs/DocumentsTab'
import type { DocumentDef } from './tabs/DocumentsTab'
import { InputsTab } from './tabs/InputsTab'
import { SettingsTab } from './tabs/SettingsTab'

type DocumenterNodeData = {
  label: string
  documentNames: string[]
  upstreamStepNames: string[]
  promptValue: string
  documents: DocumentDef[]
  modelId: string | null
  agentName: string | null
}

function DocumenterNodeComponent({ data, selected }: NodeProps) {
  const [activeTabId, setActiveTabId] = useState('prompt')
  const nodeData = data as DocumenterNodeData

  const promptValue = nodeData.promptValue
  const documents = nodeData.documents
  const upstreamStepNames = nodeData.upstreamStepNames

  const handlePromptChange = useCallback((_value: string) => {
    // Will be wired to state management in a future step
  }, [])

  const handleAddDocument = useCallback(() => {
    // Will be wired to state management in a future step
  }, [])

  const handleRemoveDocument = useCallback((_id: string) => {
    // Will be wired to state management in a future step
  }, [])

  const accentColor = PROTOCOL_TYPE_COLORS['documenter']

  const tabs: CanvasFormTab[] = [
    {
      id: 'prompt',
      icon: EditOutlined,
      tooltip: 'Prompt',
      content: <PromptTab value={promptValue} onChange={handlePromptChange} />,
    },
    {
      id: 'documents',
      icon: DescriptionOutlined,
      tooltip: 'Documents',
      content: (
        <DocumentsTab
          documents={documents}
          onAdd={handleAddDocument}
          onRemove={handleRemoveDocument}
        />
      ),
    },
    {
      id: 'inputs',
      icon: InputOutlined,
      tooltip: 'Inputs',
      content: <InputsTab upstreamStepNames={upstreamStepNames} />,
    },
    {
      id: 'settings',
      icon: SettingsOutlined,
      tooltip: 'Settings',
      content: <SettingsTab modelId={nodeData.modelId} agentName={nodeData.agentName} />,
    },
  ]

  return (
    <CanvasFormNode
      header={
        <DocumenterHeader
          name={nodeData.label}
          documentNames={nodeData.documentNames}
        />
      }
      tabs={tabs}
      activeTabId={activeTabId}
      onTabChange={setActiveTabId}
      selected={selected === true}
      accentColor={accentColor}
    />
  )
}

const DocumenterNode = memo(DocumenterNodeComponent)

export { DocumenterNode }
export type { DocumenterNodeData }
