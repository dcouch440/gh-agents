import { useContext } from 'react'
import { OutputSchemaContext } from '@/contexts/OutputSchemaContext'

const useOutputSchemaContext = () => {
  const ctx = useContext(OutputSchemaContext)
  if (!ctx) throw new Error('useOutputSchemaContext must be used within OutputSchemaProvider')
  return ctx
}

export { useOutputSchemaContext }
