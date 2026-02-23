/**
 * Compute perpendicular offsets for parallel tracks sharing a corridor.
 * Centers the group around 0 with `spacing` between each track.
 *
 * @example
 * assignParallelTracks(3, 8) // → [-8, 0, 8]
 * assignParallelTracks(2, 10) // → [-5, 5]
 * assignParallelTracks(1, 8) // → [0]
 */
const assignParallelTracks = (count: number, spacing: number): number[] => {
  if (count <= 0) return []
  if (count === 1) return [0]

  const totalWidth = (count - 1) * spacing
  const offsets: number[] = []
  for (let i = 0; i < count; i++) {
    offsets.push(-totalWidth / 2 + i * spacing)
  }
  return offsets
}

export { assignParallelTracks }
