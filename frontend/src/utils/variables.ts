/**
 * Variable extraction and resolution utilities
 *
 * Ports the backend workflow variable resolution logic from Rust to TypeScript.
 * Reference: src/server/executors/dag/mod.rs lines 165-243
 *
 * Supports the same syntax as backend workflows:
 * - {variable} → simple variable
 * - {variable.nested.path} → dot-path navigation
 * - {array.0.field} → array index access
 * - {items.$.field} → for-each current element (special case)
 */

/**
 * Extract unique root variable names from a template string.
 *
 * Examples:
 * - "{output.items}" and "{output.total}" → ["output"]
 * - "{user}" and "{admin.name}" → ["admin", "user"]
 * - "No variables" → []
 *
 * Strategy: Extract root variables only (deduplicate by root).
 * This simplifies the UX - one tab per root variable, not one per full path.
 *
 * @param template - Template string with {variable} placeholders
 * @returns Sorted array of unique root variable names
 */
export function extractVariables(template: string): string[] {
  // Regex matches: {variable} or {variable.nested.path.0.field} or {items.$.field}
  // Must start with letter or underscore, followed by alphanumeric/underscore
  // Also supports $ for for-each current element syntax
  const regex = /\{([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z0-9_$]+)*)\}/g
  const roots = new Set<string>()

  let match
  while ((match = regex.exec(template)) !== null) {
    const fullPath = match[1] // e.g., "output.items.0.name" or "items.$.name"
    const rootVar = fullPath.split('.')[0] // e.g., "output" or "items"
    roots.add(rootVar)
  }

  return Array.from(roots).sort()
}

/**
 * Resolve {variable} references in a template using mock data.
 *
 * Supports dot-path navigation for nested JSON structures:
 * - {output.items} → navigates to .items in output JSON
 * - {features.0.name} → array index + field access
 * - {data.nested.deep.value} → arbitrary depth
 *
 * Unresolved variables are left as-is: {missing} remains "{missing}"
 * This matches backend behavior (no errors on missing variables).
 *
 * @param template - Template string with {variable} placeholders
 * @param mockData - Map of variable name → JSON string
 * @returns Template with variables resolved
 */
export function resolveVariables(template: string, mockData: Record<string, string>): string {
  return template.replace(/\{([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z0-9_]+)*)\}/g, (match, path: string) => {
    const resolved = resolvePath(path, mockData)
    return resolved ?? match // Keep {variable} if unresolved
  })
}

/**
 * Navigate a dot-path into the mock data map.
 *
 * Example: "output.items.0.name"
 * 1. Look up mockData["output"] → get JSON string
 * 2. Parse JSON
 * 3. Navigate .items (object field)
 * 4. Navigate [0] (array index)
 * 5. Navigate .name (object field)
 * 6. Return stringified result
 *
 * @param path - Dot-separated path (e.g., "output.items.0.name")
 * @param mockData - Map of variable name → JSON string
 * @returns Resolved value as string, or null if unresolved
 */
function resolvePath(path: string, mockData: Record<string, string>): string | null {
  const parts = path.split('.')
  const rootVar = parts[0]
  if (!rootVar) {
    return null // Empty path
  }

  // Get root JSON string
  const jsonText = mockData[rootVar]
  if (!jsonText?.trim()) {
    return null // Unresolved
  }

  // Parse JSON
  let root: unknown
  try {
    root = JSON.parse(jsonText) as unknown
  } catch {
    return null // Invalid JSON → unresolved
  }

  // Navigate dot-path
  let current: unknown = root
  for (let i = 1; i < parts.length; i++) {
    const part = parts[i]

    // Try as array index first
    const idx = parseInt(part, 10)
    if (!isNaN(idx) && Array.isArray(current)) {
      current = current[idx]
    } else if (isObject(current)) {
      current = current[part]
    } else {
      return null // Can't navigate further
    }

    if (current === undefined || current === null) {
      return null // Path doesn't exist
    }
  }

  // Stringify result
  if (typeof current === 'string') {
    return current
  }
  if (typeof current === 'number' || typeof current === 'boolean') {
    return String(current)
  }
  // Objects/arrays → JSON string
  return JSON.stringify(current)
}

/**
 * Type guard for plain objects
 */
function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/**
 * Validate that variable mock data is valid JSON.
 *
 * Used to show errors in UI before attempting resolution.
 * Empty strings are considered valid (variable will be unresolved).
 *
 * @param jsonText - JSON string to validate
 * @returns Validation result with error message if invalid
 */
export function validateVariableData(jsonText: string): { valid: boolean; error?: string } {
  const trimmed = jsonText.trim()

  if (!trimmed) {
    return { valid: true } // Empty is valid (unresolved)
  }

  try {
    JSON.parse(trimmed)
    return { valid: true }
  } catch (e) {
    const error = e instanceof Error ? e.message : 'Invalid JSON'
    return { valid: false, error }
  }
}
