type SelectionNode = {
  id: string
  data: Record<string, unknown>
}

const EMPTY_SET: ReadonlySet<string> = new Set()

const computeHighlightedProtocolIds = (nodes: SelectionNode[]): ReadonlySet<string> => {
  let result: Set<string> | null = null
  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i]!
    if (node.data.isProtocol === true) {
      result ??= new Set()
      result.add(node.id)
    }
  }
  return result ?? EMPTY_SET
}

export { computeHighlightedProtocolIds }
export type { SelectionNode }
