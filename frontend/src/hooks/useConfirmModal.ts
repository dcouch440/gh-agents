import {useState, useCallback, useRef} from 'react'

type ConfirmModalState = {
  open: boolean
  title: string
  message: string
  confirmText: string
  cancelText: string
  confirmColor: 'primary' | 'error' | 'warning' | 'success'
  loading: boolean
  error: string | null
}

type ConfirmOptions = {
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  confirmColor?: 'primary' | 'error' | 'warning' | 'success'
}

type UseConfirmModalReturn = {
  // State
  open: boolean
  title: string
  message: string
  confirmText: string
  cancelText: string
  confirmColor: 'primary' | 'error' | 'warning' | 'success'
  loading: boolean
  error: string | null

  // Actions
  openConfirm: (options: ConfirmOptions) => void
  closeConfirm: () => void
  onConfirm: () => Promise<void>
  confirmAsync: (action: () => Promise<void>) => Promise<boolean>
}

const useConfirmModal = (): UseConfirmModalReturn => {
  const [state, setState] = useState<ConfirmModalState>({
    open: false,
    title: '',
    message: '',
    confirmText: 'Confirm',
    cancelText: 'Cancel',
    confirmColor: 'primary',
    loading: false,
    error: null,
  })

  const pendingActionRef = useRef<(() => Promise<void>) | null>(null)
  const resolveRef = useRef<((value: boolean) => void) | null>(null)

  const openConfirm = useCallback((options: ConfirmOptions) => {
    setState({
      open: true,
      title: options.title,
      message: options.message,
      confirmText: options.confirmText ?? 'Confirm',
      cancelText: options.cancelText ?? 'Cancel',
      confirmColor: options.confirmColor ?? 'primary',
      loading: false,
      error: null,
    })
  }, [])

  const closeConfirm = useCallback(() => {
    setState((prev) => ({...prev, open: false, loading: false, error: null}))
    pendingActionRef.current = null
    resolveRef.current = null
  }, [])

  const onConfirm = useCallback(async () => {
    const action = pendingActionRef.current
    const resolve = resolveRef.current

    if (!action || !resolve) {
      return
    }

    setState((prev) => ({...prev, loading: true, error: null}))
    try {
      await action()
      setState((prev) => ({...prev, open: false, loading: false}))
      resolve(true)
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Operation failed'
      setState((prev) => ({...prev, loading: false, error: errorMsg}))
      resolve(false)
    }
  }, [])

  const confirmAsync = useCallback(
    async (action: () => Promise<void>): Promise<boolean> => {
      return new Promise((resolve) => {
        pendingActionRef.current = action
        resolveRef.current = resolve
      })
    },
    [],
  )

  return {
    open: state.open,
    title: state.title,
    message: state.message,
    confirmText: state.confirmText,
    cancelText: state.cancelText,
    confirmColor: state.confirmColor,
    loading: state.loading,
    error: state.error,
    openConfirm,
    closeConfirm,
    onConfirm,
    confirmAsync,
  }
}

export {useConfirmModal}
export type {UseConfirmModalReturn, ConfirmOptions}
