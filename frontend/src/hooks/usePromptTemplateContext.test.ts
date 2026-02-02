import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { usePromptTemplateContext } from './usePromptTemplateContext'

describe('usePromptTemplateContext', () => {
  it('throws error when used outside PromptTemplateProvider', () => {
    expect(() => renderHook(() => usePromptTemplateContext())).toThrow(
      'usePromptTemplateContext must be used within PromptTemplateProvider',
    )
  })
})
