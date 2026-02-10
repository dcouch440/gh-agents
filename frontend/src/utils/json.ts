const formatJson = (text: string): string => {
  try {
    return JSON.stringify(JSON.parse(text), null, 2)
  } catch {
    return text
  }
}

const validateJsonObject = (text: string): { valid: boolean; error?: string; parsed?: Record<string, unknown> } => {
  try {
    const parsed: unknown = JSON.parse(text)
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
      return { valid: false, error: 'JSON must be an object' }
    }
    return { valid: true, parsed: parsed as Record<string, unknown> }
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
    return { valid: false, error: errorMessage }
  }
}

export { formatJson, validateJsonObject }
