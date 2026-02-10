import { memo, useState, useCallback } from 'react'
import type { NodeProps } from '@xyflow/react'
import EditOutlined from '@mui/icons-material/EditOutlined'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import InputOutlined from '@mui/icons-material/InputOutlined'
import TimelineOutlined from '@mui/icons-material/TimelineOutlined'
import { workflowStore } from '@/stores'
import { CanvasFormNode } from '../CanvasFormNode'
import type { CanvasFormTab } from '../CanvasFormNode'
import { PROTOCOL_TYPE_COLORS } from '../constants'
import { nodeDataEqual } from '../mappers'
import { DocumenterHeader } from './DocumenterHeader'
import { PromptTab } from './tabs/PromptTab'
import { DocumentsTab } from './tabs/DocumentsTab'
import type { DocumentDef } from './tabs/DocumentsTab'
import { InputsTab } from './tabs/InputsTab'
import { ActivityTab } from './tabs/SettingsTab'

type DocumenterNodeData = {
  label: string
  documentNames: string[]
  upstreamStepNames: string[]
  promptValue: string
  documents: DocumentDef[]
  modelId: string | null
  agentName: string | null
}

function DocumenterNodeComponent({ id, data, selected }: NodeProps) {
  const [activeTabId, setActiveTabId] = useState('prompt')
  const nodeData = data as DocumenterNodeData

  const promptValue = nodeData.promptValue
  const documents = nodeData.documents
  const upstreamStepNames = nodeData.upstreamStepNames

  const handlePromptChange = useCallback((value: string) => {
    workflowStore.patchStepLocal(id, { prompt_template: value })
  }, [id])

  const handleAddDocument = useCallback(() => {
    // TODO: Wire to document def creation dialog
  }, [])

  const handleRemoveDocument = useCallback((_id: string) => {
    // TODO: Wire to document def deletion
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
      id: 'activity',
      icon: TimelineOutlined,
      tooltip: 'Activity',
      content: <ActivityTab />,
    },
  ]

  return (
    <CanvasFormNode
      header={
        <DocumenterHeader
          name={nodeData.label}
          documentNames={nodeData.documentNames}
          documentCount={nodeData.documents.length}
          modelId={nodeData.modelId}
          agentName={nodeData.agentName}
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

const documenterNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected &&
  prev.id === next.id &&
  nodeDataEqual(prev.data, next.data)

const DocumenterNode = memo(DocumenterNodeComponent, documenterNodeEqual)

export { DocumenterNode }
export type { DocumenterNodeData }
