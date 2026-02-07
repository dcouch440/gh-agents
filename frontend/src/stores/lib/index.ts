export { createStore } from './createStore'
export { useStore } from './useStore'
export { shallow } from './shallow'
export { batch } from './batch'
export { createResourceStore } from './createResourceStore'
export {
  createNormalizedMap, toArray, nmGet, nmHas, nmSize,
  nmSet, nmDelete, nmFromArray, nmMerge,
} from './NormalizedMap'

export type { StoreApi, SetState, GetState, StateCreator, Listener } from './types'
export type { NormalizedMap } from './NormalizedMap'
export type { ResourceStoreConfig, ResourceState, ResourceStore, Identifiable } from './createResourceStore'
