import type { Page, Locator } from '@playwright/test'

/**
 * Right-click on the canvas pane at a specific position within the canvas element.
 * Coordinates are relative to the canvas container.
 */
export const rightClickCanvas = async (page: Page, x: number, y: number): Promise<void> => {
  const canvas = page.getByTestId('workflow-canvas')
  await canvas.click({ position: { x, y }, button: 'right' })
}

/**
 * Click a context menu item by its data-testid.
 */
export const clickContextMenuItem = async (page: Page, testId: string): Promise<void> => {
  await page.getByTestId(testId).click()
}

/**
 * Click a ReactFlow node by its data-id attribute (the node's ID).
 */
export const selectNode = async (page: Page, nodeId: string): Promise<Locator> => {
  const node = page.locator(`.react-flow__node[data-id="${nodeId}"]`)
  await node.click()
  return node
}

/**
 * Right-click a ReactFlow node by its data-id attribute.
 */
export const rightClickNode = async (page: Page, nodeId: string): Promise<void> => {
  const node = page.locator(`.react-flow__node[data-id="${nodeId}"]`)
  await node.click({ button: 'right' })
}

/**
 * Type text into a CodeMirror editor within a parent locator.
 * CodeMirror uses contenteditable .cm-content — standard fill() doesn't work.
 */
export const typeInCodeMirror = async (parent: Locator, text: string): Promise<void> => {
  const cmContent = parent.locator('.cm-content')
  await cmContent.click()
  await cmContent.page().keyboard.type(text)
}

/**
 * Select all text in a CodeMirror editor and replace it.
 */
export const replaceInCodeMirror = async (parent: Locator, text: string): Promise<void> => {
  const cmContent = parent.locator('.cm-content')
  await cmContent.click()
  const page = cmContent.page()
  await page.keyboard.press('Meta+a')
  await page.keyboard.type(text)
}

/**
 * Wait for the workflow canvas to be fully loaded and interactive.
 */
export const waitForCanvas = async (page: Page): Promise<void> => {
  await page.getByTestId('workflow-canvas').waitFor()
  // Wait for ReactFlow to initialize (the pane element appears when ready)
  await page.locator('.react-flow__pane').waitFor()
}
