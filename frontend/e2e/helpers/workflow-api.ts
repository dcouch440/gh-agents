import type { APIRequestContext } from '@playwright/test'

const API_BASE = 'http://localhost:5173/api'

type Workflow = {
  id: string
  name: string
  description: string
}

type WorkflowStep = {
  id: string
  workflow_id: string
  name: string
  execution_mode: string
  prompt_template: string
  position_x: number
  position_y: number
}

type CreateStepBody = {
  name?: string
  execution_mode?: string
  prompt_template?: string
  position_x?: number
  position_y?: number
}

/**
 * Create a test workflow via the real API.
 * Retries once on 5xx errors (backend can be briefly overloaded during test runs).
 */
export const createTestWorkflow = async (
  request: APIRequestContext,
  token: string,
  name = 'E2E Test Workflow',
): Promise<Workflow> => {
  for (let attempt = 0; attempt < 2; attempt++) {
    const res = await request.post(`${API_BASE}/workflows`, {
      headers: { Authorization: `Bearer ${token}` },
      data: { name, description: 'Created by Playwright E2E tests' },
    })

    if (res.ok()) {
      return await res.json() as Workflow
    }

    if (res.status() >= 500 && attempt === 0) {
      // Server may be overloaded — wait briefly and retry once
      await new Promise((r) => setTimeout(r, 1000))
      continue
    }

    const text = await res.text()
    throw new Error(`Failed to create workflow: ${res.status()} ${text}`)
  }

  throw new Error('Failed to create workflow after retries')
}

/**
 * Create a step on a workflow via the real API.
 */
export const createTestStep = async (
  request: APIRequestContext,
  token: string,
  workflowId: string,
  body: CreateStepBody,
): Promise<WorkflowStep> => {
  const res = await request.post(`${API_BASE}/workflows/${workflowId}/steps`, {
    headers: { Authorization: `Bearer ${token}` },
    data: body,
  })

  if (!res.ok()) {
    const text = await res.text()
    throw new Error(`Failed to create step: ${res.status()} ${text}`)
  }

  return await res.json() as WorkflowStep
}

/**
 * Delete a test workflow (cascades to steps/edges).
 */
export const deleteTestWorkflow = async (
  request: APIRequestContext,
  token: string,
  workflowId: string,
): Promise<void> => {
  await request.delete(`${API_BASE}/workflows/${workflowId}`, {
    headers: { Authorization: `Bearer ${token}` },
  })
}
