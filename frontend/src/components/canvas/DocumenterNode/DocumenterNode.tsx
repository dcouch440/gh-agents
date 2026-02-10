import { memo, useState, useCallback, useEffect } from 'react'
import { Handle, Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import { useTheme } from '@mui/material/styles'
import EditOutlined from '@mui/icons-material/EditOutlined'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import InputOutlined from '@mui/icons-material/InputOutlined'
import SettingsOutlined from '@mui/icons-material/SettingsOutlined'
import { useStore, workflowStore } from '@/stores'
import type { CreateDocumentDefRequest } from '@/types/workflow'
import { CanvasFormNode } from '../CanvasFormNode'
import type { CanvasFormTab } from '../CanvasFormNode'
import { CANVAS, PROTOCOL_TYPE_COLORS } from '../constants'
import { nodeDataEqual } from '../mappers'
import { DocumenterHeader } from './DocumenterHeader'
import { PromptTab } from './tabs/PromptTab'
import { DocumentsTab } from './tabs/DocumentsTab'
import { InputsTab } from './tabs/InputsTab'
import { SettingsTab } from './tabs/SettingsTab'

type DocumenterNodeData = {
  label: string
  documentNames: string[]
  upstreamStepNames: string[]
  promptValue: string
  modelId: string | null
  agentName: string | null
}

function DocumenterNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const [activeTabId, setActiveTabId] = useState('prompt')
  const [adding, setAdding] = useState(false)
  const nodeData = data as DocumenterNodeData

  const promptValue = nodeData.promptValue
  const upstreamStepNames = nodeData.upstreamStepNames

  const documentDefs = useStore(workflowStore.store, workflowStore.selectStepDocumentDefs(id))

  useEffect(() => {
    void workflowStore.fetchDocumentDefs(id)
  }, [id])

  const handlePromptChange = useCallback(
    (value: string) => {
      workflowStore.patchStepLocal(id, { prompt_template: value })
    },
    [id],
  )

  const handleAddDocument = useCallback(() => {
    setAdding(true)
  }, [])

  const handleSubmitNew = useCallback(
    (body: CreateDocumentDefRequest) => {
      void workflowStore.createDocumentDef(id, body)
      setAdding(false)
    },
    [id],
  )

  const handleCancelAdd = useCallback(() => {
    setAdding(false)
  }, [])

  const handleRemoveDocument = useCallback(
    (defId: string) => {
      void workflowStore.deleteDocumentDef(id, defId)
    },
    [id],
  )

  const handleNameChange = useCallback(
    (value: string) => {
      workflowStore.patchStepLocal(id, { name: value })
    },
    [id],
  )

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
          documents={documentDefs}
          adding={adding}
          onAdd={handleAddDocument}
          onSubmitNew={handleSubmitNew}
          onCancelAdd={handleCancelAdd}
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
      content: <SettingsTab name={nodeData.label} onNameChange={handleNameChange} />,
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
      extraHandles={
        <Handle
          type="source"
          position={Position.Top}
          id="documents"
          style={{
            width: CANVAS.HANDLE_SIZE,
            height: CANVAS.HANDLE_SIZE,
            background: accentColor,
            border: `2px solid ${theme.palette.custom.bgHeader}`,
          }}
        />
      }
    />
  )
}

const documenterNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const DocumenterNode = memo(DocumenterNodeComponent, documenterNodeEqual)

export { DocumenterNode }
export type { DocumenterNodeData }
