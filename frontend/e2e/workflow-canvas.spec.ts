import { test, expect } from '@playwright/test'
import { getAuthToken, setupAuth } from './helpers/auth'
import { createTestWorkflow, createTestStep, deleteTestWorkflow } from './helpers/workflow-api'
import { rightClickCanvas, clickContextMenuItem, rightClickNode, waitForCanvas } from './helpers/canvas'

let token: string
let workflowId: string

test.beforeAll(async ({ request }) => {
  token = await getAuthToken(request)
})

test.describe('Workflow Canvas — Node Placement', () => {
  test.beforeEach(async ({ page, request }) => {
    const workflow = await createTestWorkflow(request, token)
    workflowId = workflow.id
    await setupAuth(page, token)
    await page.goto(`/workflows/${workflowId}`)
    await waitForCanvas(page)
  })

  test.afterEach(async ({ request }) => {
    await deleteTestWorkflow(request, token, workflowId)
  })

  test('place an LLM step node via context menu', async ({ page }) => {
    await rightClickCanvas(page, 400, 300)
    await expect(page.getByTestId('canvas-context-menu')).toBeVisible()

    await clickContextMenuItem(page, 'ctx-add-single')
    await expect(page.getByTestId('canvas-context-menu')).not.toBeVisible()

    // Wait for the node to appear on canvas
    await expect(page.locator('.react-flow__node')).toHaveCount(1, { timeout: 5000 })
  })

  test('place a For-Each step node via context menu', async ({ page }) => {
    await rightClickCanvas(page, 400, 300)
    await clickContextMenuItem(page, 'ctx-add-for_each')

    await expect(page.locator('.react-flow__node')).toHaveCount(1, { timeout: 5000 })
  })

  test('place a Documenter node via context menu', async ({ page }) => {
    await rightClickCanvas(page, 400, 300)
    await clickContextMenuItem(page, 'ctx-add-documenter')

    await expect(page.locator('.react-flow__node')).toHaveCount(1, { timeout: 5000 })
  })

  test('place multiple node types on canvas', async ({ page }) => {
    // Place LLM step
    await rightClickCanvas(page, 200, 200)
    await clickContextMenuItem(page, 'ctx-add-single')
    await expect(page.locator('.react-flow__node')).toHaveCount(1, { timeout: 5000 })

    // Place For-Each step
    await rightClickCanvas(page, 500, 200)
    await clickContextMenuItem(page, 'ctx-add-for_each')
    await expect(page.locator('.react-flow__node')).toHaveCount(2, { timeout: 5000 })

    // Place Documenter (right-click near bottom — tests viewport clamping)
    await rightClickCanvas(page, 200, 400)
    await clickContextMenuItem(page, 'ctx-add-documenter')
    await expect(page.locator('.react-flow__node')).toHaveCount(3, { timeout: 5000 })
  })
})

test.describe('Workflow Canvas — Node Deletion', () => {
  let stepId: string

  test.beforeEach(async ({ page, request }) => {
    const workflow = await createTestWorkflow(request, token)
    workflowId = workflow.id

    // Pre-create a step so we have something to delete
    const step = await createTestStep(request, token, workflowId, {
      name: 'Step to Delete',
      execution_mode: 'single',
      position_x: 300,
      position_y: 300,
    })
    stepId = step.id

    await setupAuth(page, token)
    await page.goto(`/workflows/${workflowId}`)
    await waitForCanvas(page)

    // Wait for the pre-created node to render
    await expect(page.locator('.react-flow__node')).toHaveCount(1, { timeout: 5000 })
  })

  test.afterEach(async ({ request }) => {
    await deleteTestWorkflow(request, token, workflowId)
  })

  test('delete a node via Backspace key', async ({ page }) => {
    // Click the node to select it
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)
    await node.click()

    // Press Backspace to delete
    await page.keyboard.press('Backspace')

    await expect(page.locator('.react-flow__node')).toHaveCount(0, { timeout: 5000 })
  })

  test('delete a node via Delete key', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)
    await node.click()

    await page.keyboard.press('Delete')

    await expect(page.locator('.react-flow__node')).toHaveCount(0, { timeout: 5000 })
  })

  test('delete a node via context menu', async ({ page }) => {
    await rightClickNode(page, stepId)

    await expect(page.getByTestId('ctx-delete-step')).toBeVisible()
    await clickContextMenuItem(page, 'ctx-delete-step')

    await expect(page.locator('.react-flow__node')).toHaveCount(0, { timeout: 5000 })
  })
})

test.describe('Workflow Canvas — Node Selection', () => {
  let stepAId: string
  let stepBId: string

  test.beforeEach(async ({ page, request }) => {
    const workflow = await createTestWorkflow(request, token)
    workflowId = workflow.id

    // Pre-create two steps at well-separated positions
    const stepA = await createTestStep(request, token, workflowId, {
      name: 'Step A',
      execution_mode: 'single',
      position_x: 200,
      position_y: 200,
    })
    const stepB = await createTestStep(request, token, workflowId, {
      name: 'Step B',
      execution_mode: 'single',
      position_x: 600,
      position_y: 200,
    })
    stepAId = stepA.id
    stepBId = stepB.id

    await setupAuth(page, token)
    await page.goto(`/workflows/${workflowId}`)
    await waitForCanvas(page)

    // Wait for both nodes to render
    await expect(page.locator('.react-flow__node')).toHaveCount(2, { timeout: 5000 })
  })

  test.afterEach(async ({ request }) => {
    await deleteTestWorkflow(request, token, workflowId)
  })

  test('clicking between nodes toggles selection correctly', async ({ page }) => {
    const nodeA = page.locator(`.react-flow__node[data-id="${stepAId}"]`)
    const nodeB = page.locator(`.react-flow__node[data-id="${stepBId}"]`)

    // Click node A — should be selected, B should not
    await nodeA.click()
    await expect(nodeA).toHaveClass(/selected/)
    await expect(nodeB).not.toHaveClass(/selected/)

    // Click node B — should be selected, A should not
    await nodeB.click()
    await expect(nodeB).toHaveClass(/selected/)
    await expect(nodeA).not.toHaveClass(/selected/)

    // Click node A again
    await nodeA.click()
    await expect(nodeA).toHaveClass(/selected/)
    await expect(nodeB).not.toHaveClass(/selected/)

    // Click node B again
    await nodeB.click()
    await expect(nodeB).toHaveClass(/selected/)
    await expect(nodeA).not.toHaveClass(/selected/)

    // Click node A one more time (5 total switches)
    await nodeA.click()
    await expect(nodeA).toHaveClass(/selected/)
    await expect(nodeB).not.toHaveClass(/selected/)
  })
})
