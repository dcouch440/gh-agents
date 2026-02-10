import { test, expect } from '@playwright/test'
import { getAuthToken, setupAuth } from './helpers/auth'
import { createTestWorkflow, createTestStep, deleteTestWorkflow } from './helpers/workflow-api'
import { waitForCanvas } from './helpers/canvas'

let token: string
let workflowId: string
let stepId: string

test.beforeAll(async ({ request }) => {
  token = await getAuthToken(request)
})

test.describe('Documenter Node', () => {
  test.beforeEach(async ({ page, request }) => {
    const workflow = await createTestWorkflow(request, token)
    workflowId = workflow.id

    const step = await createTestStep(request, token, workflowId, {
      name: 'Test Documenter',
      execution_mode: 'documenter',
      prompt_template: 'Write a summary of the topic',
      position_x: 200,
      position_y: 200,
    })
    stepId = step.id

    await setupAuth(page, token)
    await page.goto(`/workflows/${workflowId}`)
    await waitForCanvas(page)
    await expect(page.locator('.react-flow__node')).toHaveCount(1, { timeout: 5000 })
  })

  test.afterEach(async ({ request }) => {
    await deleteTestWorkflow(request, token, workflowId)
  })

  test('displays documenter node with header text', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)
    await expect(node).toContainText('Test Documenter')
  })

  test('prompt tab is active by default', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)

    const promptTab = node.getByTestId('tab-prompt')
    await expect(promptTab).toHaveAttribute('aria-selected', 'true')

    // CodeMirror editor should be visible
    await expect(node.locator('.cm-content')).toBeVisible()
  })

  test('switch to Documents tab', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)

    await node.getByTestId('tab-documents').click()
    await expect(node.getByTestId('tab-documents')).toHaveAttribute('aria-selected', 'true')
    await expect(node.getByTestId('tab-prompt')).toHaveAttribute('aria-selected', 'false')
  })

  test('switch to Inputs tab', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)

    await node.getByTestId('tab-inputs').click()
    await expect(node.getByTestId('tab-inputs')).toHaveAttribute('aria-selected', 'true')
    await expect(node.getByTestId('tab-prompt')).toHaveAttribute('aria-selected', 'false')
  })

  test('switch to Activity tab', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)

    await node.getByTestId('tab-activity').click()
    await expect(node.getByTestId('tab-activity')).toHaveAttribute('aria-selected', 'true')
    await expect(node.getByTestId('tab-prompt')).toHaveAttribute('aria-selected', 'false')
  })

  test('cycle through all four tabs', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)

    // Start on Prompt (default)
    await expect(node.getByTestId('tab-prompt')).toHaveAttribute('aria-selected', 'true')

    // Switch to Documents
    await node.getByTestId('tab-documents').click()
    await expect(node.getByTestId('tab-documents')).toHaveAttribute('aria-selected', 'true')
    await expect(node.getByTestId('tab-prompt')).toHaveAttribute('aria-selected', 'false')

    // Switch to Inputs
    await node.getByTestId('tab-inputs').click()
    await expect(node.getByTestId('tab-inputs')).toHaveAttribute('aria-selected', 'true')
    await expect(node.getByTestId('tab-documents')).toHaveAttribute('aria-selected', 'false')

    // Switch to Activity
    await node.getByTestId('tab-activity').click()
    await expect(node.getByTestId('tab-activity')).toHaveAttribute('aria-selected', 'true')
    await expect(node.getByTestId('tab-inputs')).toHaveAttribute('aria-selected', 'false')

    // Back to Prompt
    await node.getByTestId('tab-prompt').click()
    await expect(node.getByTestId('tab-prompt')).toHaveAttribute('aria-selected', 'true')
    await expect(node.getByTestId('tab-activity')).toHaveAttribute('aria-selected', 'false')
  })
})
