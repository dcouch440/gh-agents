type ModelSpendRow = {
  model_id: string
  total_input_tokens: number
  total_output_tokens: number
  total_cost_usd: number
  call_count: number
}

type CostResponse = {
  total_spend: number
  models: ModelSpendRow[]
}

export type { ModelSpendRow, CostResponse }
