import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import { useWorkflowContext } from '@/hooks/useWorkflowContext'
import type {
  Workflow,
  WorkflowStep,
  WorkflowStepEdge,
  StepDocument,
  CreateWorkflowRequest,
  UpdateWorkflowRequest,
  CreateStepRequest,
  UpdateStepRequest,
  EdgeRequest,
  StepDocumentRequest,
} from '@/types/workflow'

const useCreateWorkflow = () => {
  const { reload } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: CreateWorkflowRequest): Promise<Workflow> => {
    setLoading(true)
    setError(null)
    try {
      const workflow = await api.post<Workflow>(API.WORKFLOWS, body)
      reload()
      return workflow
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create workflow'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useUpdateWorkflow = () => {
  const { reload } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string, body: UpdateWorkflowRequest): Promise<Workflow> => {
    setLoading(true)
    setError(null)
    try {
      const workflow = await api.put<Workflow>(API.WORKFLOW(id), body)
      reload()
      return workflow
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update workflow'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useDeleteWorkflow = () => {
  const { reload } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.WORKFLOW(id))
      reload()
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete workflow'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useCreateStep = () => {
  const { loadWorkflow } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (workflowId: string, body: CreateStepRequest): Promise<WorkflowStep> => {
    setLoading(true)
    setError(null)
    try {
      const step = await api.post<WorkflowStep>(API.WORKFLOW_STEPS(workflowId), body)
      await loadWorkflow(workflowId)
      return step
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create step'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [loadWorkflow])

  return { mutate, loading, error }
}

const useUpdateStep = () => {
  const { loadWorkflow } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (workflowId: string, stepId: string, body: UpdateStepRequest): Promise<WorkflowStep> => {
    setLoading(true)
    setError(null)
    try {
      const step = await api.put<WorkflowStep>(API.WORKFLOW_STEP(workflowId, stepId), body)
      await loadWorkflow(workflowId)
      return step
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update step'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [loadWorkflow])

  return { mutate, loading, error }
}

const useDeleteStep = () => {
  const { loadWorkflow } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (workflowId: string, stepId: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.WORKFLOW_STEP(workflowId, stepId))
      await loadWorkflow(workflowId)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete step'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [loadWorkflow])

  return { mutate, loading, error }
}

const useCreateEdge = () => {
  const { loadWorkflow } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (workflowId: string, body: EdgeRequest): Promise<WorkflowStepEdge> => {
    setLoading(true)
    setError(null)
    try {
      const edge = await api.post<WorkflowStepEdge>(API.WORKFLOW_EDGES(workflowId), body)
      await loadWorkflow(workflowId)
      return edge
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create edge'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [loadWorkflow])

  return { mutate, loading, error }
}

const useDeleteEdge = () => {
  const { loadWorkflow } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (workflowId: string, edgeId: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.WORKFLOW_EDGE(workflowId, edgeId))
      await loadWorkflow(workflowId)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete edge'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [loadWorkflow])

  return { mutate, loading, error }
}

const useAddStepDocument = () => {
  const { loadWorkflow } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (workflowId: string, stepId: string, body: StepDocumentRequest): Promise<StepDocument> => {
    setLoading(true)
    setError(null)
    try {
      const doc = await api.post<StepDocument>(API.STEP_DOCUMENTS(workflowId, stepId), body)
      await loadWorkflow(workflowId)
      return doc
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to add step document'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [loadWorkflow])

  return { mutate, loading, error }
}

const useRemoveStepDocument = () => {
  const { loadWorkflow } = useWorkflowContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (workflowId: string, stepId: string, documentId: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.STEP_DOCUMENT(workflowId, stepId, documentId))
      await loadWorkflow(workflowId)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to remove step document'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [loadWorkflow])

  return { mutate, loading, error }
}

export {
  useCreateWorkflow,
  useUpdateWorkflow,
  useDeleteWorkflow,
  useCreateStep,
  useUpdateStep,
  useDeleteStep,
  useCreateEdge,
  useDeleteEdge,
  useAddStepDocument,
  useRemoveStepDocument,
}
