import { useParams } from 'react-router-dom'
import { useState } from 'react'
import { useAgent } from '@/hooks/useAgents'
import { useAgentDocuments } from '@/hooks/useAgentDocuments'
import { useDocuments } from '@/hooks/useDocuments'
import {
  PageHeader,
  Card,
  LoadingSpinner,
  ErrorMessage,
  EmptyState,
  DataTable,
  type Column,
} from '@/components/primitives'
import type { DocumentListItem } from '@/types/document'

function AgentDetailPage() {
  const { id } = useParams()
  const { agent, loading: agentLoading, error: agentError } = useAgent(id ?? null)
  const {
    documents: agentDocs,
    loading: docsLoading,
    error: docsError,
    saving,
    addDocument,
    removeDocument,
  } = useAgentDocuments(id ?? null)
  const { documents: allDocuments, loading: allDocsLoading } = useDocuments()
  const [showAddDialog, setShowAddDialog] = useState(false)

  if (!id) {
    return <ErrorMessage message="No agent ID provided" />
  }

  if (agentLoading || docsLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <LoadingSpinner size="large" />
      </div>
    )
  }

  if (agentError) {
    return <ErrorMessage message={agentError} />
  }

  if (!agent) {
    return <ErrorMessage message="Agent not found" />
  }

  const handleAddDocument = async (documentId: string) => {
    await addDocument(documentId)
    setShowAddDialog(false)
  }

  const handleRemoveDocument = async (documentId: string) => {
    if (confirm('Remove this document from agent context?')) {
      await removeDocument(documentId)
    }
  }

  const availableDocuments = allDocuments.filter(
    (doc) => !agentDocs.some((ad) => ad.id === doc.id)
  )

  const columns: Column<DocumentListItem>[] = [
    {
      key: 'title',
      header: 'Title',
      render: (doc) => doc.title,
    },
    {
      key: 'doc_type',
      header: 'Type',
      render: (doc) => doc.doc_type ?? 'N/A',
    },
    {
      key: 'ref_tag',
      header: 'Ref Tag',
      render: (doc) => doc.ref_tag ?? 'N/A',
    },
    {
      key: 'actions',
      header: 'Actions',
      render: (doc) => (
        <button
          onClick={() => {
            void handleRemoveDocument(doc.id)
          }}
          disabled={saving}
          className="btn btn--small btn--danger"
          type="button"
        >
          Remove
        </button>
      ),
    },
  ]

  return (
    <div>
      <PageHeader title={agent.name}>
        <div className="text-sm text-gray-600">{agent.model_id}</div>
      </PageHeader>

      <div className="space-y-6">
        <Card title="Agent Details">
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">System Prompt</label>
              <div className="mt-1 p-3 bg-gray-50 rounded border border-gray-200">
                <pre className="text-sm whitespace-pre-wrap">{agent.system_prompt}</pre>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700">Model Provider</label>
                <div className="mt-1 text-sm">{agent.model_provider}</div>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700">Status</label>
                <div className="mt-1 text-sm">{agent.status}</div>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700">Max Tokens</label>
                <div className="mt-1 text-sm">{agent.model_max_tokens}</div>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700">Temperature</label>
                <div className="mt-1 text-sm">{agent.model_temperature}</div>
              </div>
            </div>
          </div>
        </Card>

        <Card
          title="Agent Context Documents"
          actions={
            <button
              onClick={() => setShowAddDialog(true)}
              disabled={saving || allDocsLoading}
              className="btn btn--small btn--primary"
              type="button"
            >
              Add Document
            </button>
          }
        >
          {docsError && <ErrorMessage message={docsError} />}
          {agentDocs.length === 0 ? (
            <EmptyState message="No context documents attached to this agent" />
          ) : (
            <DataTable data={agentDocs} columns={columns} />
          )}
        </Card>
      </div>

      {showAddDialog && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 max-w-2xl w-full max-h-[80vh] overflow-y-auto">
            <h2 className="text-xl font-semibold mb-4">Add Document to Agent Context</h2>
            {availableDocuments.length === 0 ? (
              <EmptyState message="All documents are already attached to this agent" />
            ) : (
              <div className="space-y-2">
                {availableDocuments.map((doc) => (
                  <div
                    key={doc.id}
                    className="border border-gray-200 rounded p-3 hover:bg-gray-50 cursor-pointer"
                    onClick={() => {
                      void handleAddDocument(doc.id)
                    }}
                  >
                    <div className="font-medium">{doc.title}</div>
                    <div className="text-sm text-gray-600">
                      {doc.doc_type ?? 'N/A'} {doc.ref_tag ? `• ${doc.ref_tag}` : ''}
                    </div>
                  </div>
                ))}
              </div>
            )}
            <div className="mt-4 flex justify-end">
              <button
                onClick={() => setShowAddDialog(false)}
                className="btn btn--secondary"
                type="button"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export { AgentDetailPage }
