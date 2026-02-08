/**
 * JSON Schema field extraction utility
 *
 * Extracts dot-path field names from JSON Schema objects so they can be
 * offered as variable autocomplete options in prompt templates.
 */

type SchemaField = {
  path: string
  type: string
  description: string | null
}

const DEFAULT_MAX_DEPTH = 3

/**
 * Extract all leaf and intermediate fields from a JSON Schema as dot-paths.
 *
 * Handles `properties` (objects), `items` (arrays), and nested structures.
 * Stops recursing at `maxDepth` to prevent infinite loops on recursive schemas.
 */
const extractSchemaFields = (
  schema: Record<string, unknown>,
  maxDepth: number = DEFAULT_MAX_DEPTH,
): SchemaField[] => {
  const fields: SchemaField[] = []
  walkSchema(schema, '', fields, 0, maxDepth)
  return fields
}

const walkSchema = (
  node: Record<string, unknown>,
  prefix: string,
  out: SchemaField[],
  depth: number,
  maxDepth: number,
): void => {
  if (depth > maxDepth) return

  const nodeType = typeof node.type === 'string' ? node.type : 'object'
  const description = typeof node.description === 'string' ? node.description : null

  // If this node has properties, it's an object schema
  const properties = node.properties as Record<string, unknown> | undefined
  if (properties && typeof properties === 'object') {
    for (const [key, value] of Object.entries(properties)) {
      if (typeof value !== 'object' || value === null) continue
      const prop = value as Record<string, unknown>

      const path = prefix ? `${prefix}.${key}` : key
      const propType = typeof prop.type === 'string' ? prop.type : 'unknown'
      const propDesc = typeof prop.description === 'string' ? prop.description : null

      out.push({ path, type: propType, description: propDesc })

      // Recurse into nested objects
      if (propType === 'object') {
        walkSchema(prop, path, out, depth + 1, maxDepth)
      }

      // Recurse into array items
      if (propType === 'array') {
        const items = prop.items as Record<string, unknown> | undefined
        if (items && typeof items === 'object') {
          walkSchema(items, path, out, depth + 1, maxDepth)
        }
      }
    }
    return
  }

  // Array with items schema (when we're already inside an array)
  const items = node.items as Record<string, unknown> | undefined
  if (nodeType === 'array' && items && typeof items === 'object') {
    if (prefix) {
      out.push({ path: prefix, type: 'array', description })
    }
    walkSchema(items, prefix, out, depth + 1, maxDepth)
    return
  }

  // Leaf node with a prefix — add it if not already added
  if (prefix && !out.some((f) => f.path === prefix)) {
    out.push({ path: prefix, type: nodeType, description })
  }
}

export { extractSchemaFields }
export type { SchemaField }
