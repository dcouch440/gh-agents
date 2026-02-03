import type { UsageSummary } from '@/types'

type TokenUsageStatusProps = {
  usage: UsageSummary[]
}

const fmtTokens = (n: number): string => {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return `${n}`
}

function TokenUsageStatus({ usage }: TokenUsageStatusProps) {
  const totalInput = usage.reduce((s, r) => s + r.total_input, 0)
  const totalOutput = usage.reduce((s, r) => s + r.total_output, 0)
  const totalCalls = usage.reduce((s, r) => s + r.call_count, 0)

  return (
    <div className="token-usage">
      <div className="token-usage__row token-usage__row--header">
        <span className="token-usage__cell token-usage__cell--model">MODEL</span>
        <span className="token-usage__cell token-usage__cell--num">CALLS</span>
        <span className="token-usage__cell token-usage__cell--num">IN</span>
        <span className="token-usage__cell token-usage__cell--num">OUT</span>
      </div>

      {usage.map((row) => (
        <div key={row.model_id} className="token-usage__row">
          <span className="token-usage__cell token-usage__cell--model">{row.model_id}</span>
          <span className="token-usage__cell token-usage__cell--num">{row.call_count}</span>
          <span className="token-usage__cell token-usage__cell--num">{fmtTokens(row.total_input)}</span>
          <span className="token-usage__cell token-usage__cell--num">{fmtTokens(row.total_output)}</span>
        </div>
      ))}

      <div className="token-usage__row token-usage__row--total">
        <span className="token-usage__cell token-usage__cell--model">TOTAL</span>
        <span className="token-usage__cell token-usage__cell--num">{totalCalls}</span>
        <span className="token-usage__cell token-usage__cell--num">{fmtTokens(totalInput)}</span>
        <span className="token-usage__cell token-usage__cell--num">{fmtTokens(totalOutput)}</span>
      </div>
    </div>
  )
}

export { TokenUsageStatus }
export type { TokenUsageStatusProps }
