import { test, expect } from '@playwright/test'
import { getAuthToken, setupAuth } from './helpers/auth'
import { createTestWorkflow, createTestStep, deleteTestWorkflow } from './helpers/workflow-api'
import { waitForCanvas, typeInCodeMirror } from './helpers/canvas'

let token: string
let workflowId: string
let stepId: string

test.beforeAll(async ({ request }) => {
  token = await getAuthToken(request)
})

test.describe('Save/Discard Bar', () => {
  test.beforeEach(async ({ page, request }) => {
    const workflow = await createTestWorkflow(request, token)
    workflowId = workflow.id

    const step = await createTestStep(request, token, workflowId, {
      name: 'Save Test Documenter',
      execution_mode: 'documenter',
      prompt_template: 'Original prompt text',
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

  test('save bar is hidden when no changes are made', async ({ page }) => {
    // The SaveDiscardBar returns null when not dirty, so it won't be in the DOM
    await expect(page.getByTestId('save-discard-bar')).toHaveCount(0)
  })

  test('editing prompt template shows save/discard bar', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)

    // Type into the CodeMirror editor to trigger dirty state
    await typeInCodeMirror(node, ' appended text')

    // Save/Discard bar should appear
    await expect(page.getByTestId('save-discard-bar')).toBeVisible({ timeout: 5000 })
    await expect(page.getByTestId('toolbar-save-button')).toBeVisible()
    await expect(page.getByTestId('toolbar-discard-button')).toBeVisible()
  })

  test('clicking Save persists changes and hides the bar', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)

    // Edit the prompt
    await typeInCodeMirror(node, ' -- saved edit')

    await expect(page.getByTestId('save-discard-bar')).toBeVisible({ timeout: 5000 })

    // Click Save
    await page.getByTestId('toolbar-save-button').click()

    // Bar should disappear after save
    await expect(page.getByTestId('save-discard-bar')).toHaveCount(0, { timeout: 10000 })

    // Reload the page and verify the edit persisted in the real database
    await page.reload()
    await waitForCanvas(page)
    await expect(page.locator('.react-flow__node')).toHaveCount(1, { timeout: 5000 })

    // The CodeMirror editor should contain the saved text
    const reloadedNode = page.locator(`.react-flow__node[data-id="${stepId}"]`)
    await expect(reloadedNode.locator('.cm-content')).toContainText('saved edit')
  })

  test('clicking Discard reverts changes and hides the bar', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)

    // Edit the prompt
    await typeInCodeMirror(node, ' -- discarded text')

    await expect(page.getByTestId('save-discard-bar')).toBeVisible({ timeout: 5000 })

    // Click Discard
    await page.getByTestId('toolbar-discard-button').click()

    // Bar should disappear
    await expect(page.getByTestId('save-discard-bar')).toHaveCount(0, { timeout: 10000 })

    // The original text should be back (no "discarded text")
    const cmContent = node.locator('.cm-content')
    await expect(cmContent).not.toContainText('discarded text')
  })

  test('Cmd+S keyboard shortcut triggers save', async ({ page }) => {
    const node = page.locator(`.react-flow__node[data-id="${stepId}"]`)

    // Edit the prompt
    await typeInCodeMirror(node, ' -- keyboard saved')

    await expect(page.getByTestId('save-discard-bar')).toBeVisible({ timeout: 5000 })

    // Press Cmd+S
    await page.keyboard.press('Meta+s')

    // Bar should disappear after save
    await expect(page.getByTestId('save-discard-bar')).toHaveCount(0, { timeout: 10000 })

    // Reload and verify persistence
    await page.reload()
    await waitForCanvas(page)
    await expect(page.locator('.react-flow__node')).toHaveCount(1, { timeout: 5000 })

    const reloadedNode = page.locator(`.react-flow__node[data-id="${stepId}"]`)
    await expect(reloadedNode.locator('.cm-content')).toContainText('keyboard saved')
  })
})
