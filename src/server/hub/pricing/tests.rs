#[cfg(test)]
mod tests {
    use super::super::*;

    /// Cost of 1M input + 1M output tokens, for readable assertions.
    fn per_million(model: &str) -> (f32, f32) {
        (
            compute_cost(model, 1_000_000, 0),
            compute_cost(model, 0, 1_000_000),
        )
    }

    // The regression that motivated this module: `deepseek` was in the
    // local-model list, so the hosted model billed $0.00 and the entire cost
    // surface read zero after the provider flip.
    #[test]
    fn the_hosted_deepseek_model_is_not_free() {
        let cost = compute_cost(crate::constants::MODEL_DEEPSEEK_V4_FLASH, 1_000_000, 0);
        assert!(cost > 0.0, "hosted DeepSeek must not bill as a local model");
    }

    #[test]
    fn deepseek_rates_match_deepinfra_standard_tier() {
        let (input, output) = per_million(crate::constants::MODEL_DEEPSEEK_V4_FLASH);
        assert!((input - 0.08).abs() < 1e-6, "input rate was {input}");
        assert!((output - 0.18).abs() < 1e-6, "output rate was {output}");
    }

    // GLM is on a 50% promotion as of 2026-08-29; these assert the
    // promotional rates actually billed, not the list price.
    #[test]
    fn glm_rates_match_the_deepinfra_promotional_tier() {
        let (input, output) = per_million(crate::constants::MODEL_GLM_5_3_FLASH);
        assert!((input - 0.075).abs() < 1e-6, "input rate was {input}");
        assert!((output - 0.25).abs() < 1e-6, "output rate was {output}");
    }

    #[test]
    fn glm_cached_input_is_billed_at_the_cache_rate() {
        // 1M input of which 800k cached: 200k at 0.075 + 800k at 0.015.
        let cost =
            compute_cost_cached(crate::constants::MODEL_GLM_5_3_FLASH, 1_000_000, 800_000, 0);
        let want = 0.2 * 0.075 + 0.8 * 0.015;
        assert!((cost - want).abs() < 1e-6, "got {cost}, want {want}");
    }

    /// The active tier model must never fall through to the generic
    /// $1.00/$3.00 default — that silently over-bills every run by >13x.
    #[test]
    fn the_active_tier_model_has_explicit_rates() {
        let (input, _) = per_million(crate::constants::MODEL_TIER1);
        assert!(
            (input - 0.075).abs() < 1e-6,
            "tier 1 input rate was {input}"
        );
    }

    #[test]
    fn cached_input_is_billed_at_the_cache_rate() {
        // 1M input of which 800k cached: 200k at 0.08 + 800k at 0.016.
        let cost = compute_cost_cached(
            crate::constants::MODEL_DEEPSEEK_V4_FLASH,
            1_000_000,
            800_000,
            0,
        );
        let want = 0.2 * 0.08 + 0.8 * 0.016;
        assert!((cost - want).abs() < 1e-6, "got {cost}, want {want}");
    }

    // Cached is a subset of input, not an addition. Treating it as an addition
    // would bill those tokens twice.
    #[test]
    fn fully_cached_input_costs_only_the_cache_rate() {
        let cost = compute_cost_cached(
            crate::constants::MODEL_DEEPSEEK_V4_FLASH,
            1_000_000,
            1_000_000,
            0,
        );
        assert!((cost - 0.016).abs() < 1e-6, "got {cost}");
    }

    #[test]
    fn cached_never_exceeds_input_even_if_a_provider_reports_nonsense() {
        let cost = compute_cost_cached(crate::constants::MODEL_DEEPSEEK_V4_FLASH, 100, 999_999, 0);
        let sane = compute_cost_cached(crate::constants::MODEL_DEEPSEEK_V4_FLASH, 100, 100, 0);
        assert!((cost - sane).abs() < 1e-9, "clamping failed: {cost}");
    }

    #[test]
    fn compute_cost_is_compute_cost_cached_with_no_cache() {
        let a = compute_cost(crate::constants::MODEL_DEEPSEEK_V4_FLASH, 5_000, 1_000);
        let b = compute_cost_cached(crate::constants::MODEL_DEEPSEEK_V4_FLASH, 5_000, 0, 1_000);
        assert!((a - b).abs() < 1e-9);
    }

    // Ollama serves DeepSeek builds locally and those really are free. The
    // discriminator is the hosted namespace, not the word "deepseek".
    #[test]
    fn locally_hosted_deepseek_stays_free() {
        assert_eq!(compute_cost("deepseek-coder", 1_000_000, 1_000_000), 0.0);
        assert_eq!(compute_cost("deepseek-r1:7b", 1_000_000, 1_000_000), 0.0);
    }

    #[test]
    fn namespaced_ollama_ids_are_still_free() {
        // A `/` in the id does not by itself mean a hosted model — Ollama ids
        // are free-form and routinely namespaced. A known local namespace or
        // an Ollama-style `:tag` is what marks them.
        for m in [
            "hf.co/org/qwen2:7b",
            "library/llama3",
            "myorg/mistral:latest",
        ] {
            assert_eq!(
                compute_cost(m, 1_000_000, 1_000_000),
                0.0,
                "{m} should be free"
            );
        }
    }

    // Hosted models from families that are also runnable locally must not
    // fall into the free path: the family name appears in both, so the
    // namespace is the only thing that separates them. Under-billing is
    // silent, which is why the ambiguous case bills rather than zeroes.
    #[test]
    fn hosted_models_from_locally_runnable_families_are_billed() {
        for m in [
            "meta-llama/Meta-Llama-3.1-405B-Instruct",
            "mistralai/Mistral-Small-24B-Instruct-2501",
            "google/gemma-3-27b-it",
            "microsoft/phi-4",
            "Qwen/Qwen3-235B-A22B",
        ] {
            assert!(
                compute_cost(m, 1_000_000, 1_000_000) > 0.0,
                "{m} should be billed, not free"
            );
        }
    }

    #[test]
    fn other_deepinfra_deepseek_models_are_also_billed() {
        // Guards the DEEPINFRA_MODEL override: any model in the hosted
        // namespace must bill, not just the one we default to.
        assert!(compute_cost("deepseek-ai/DeepSeek-Some-Other", 1_000_000, 0) > 0.0);
    }

    #[test]
    fn existing_provider_rates_are_unchanged() {
        assert_eq!(per_million("claude-opus-4-5-20251101"), (15.0, 75.0));
        assert_eq!(per_million("claude-sonnet-4-20250514"), (3.0, 15.0));
        assert_eq!(per_million("claude-3-5-haiku-20241022"), (0.25, 1.25));
        assert_eq!(per_million("grok-4-0709"), (3.0, 12.0));
        assert_eq!(per_million("grok-4-1-fast-reasoning"), (2.0, 8.0));
        assert_eq!(per_million("grok-4-1-fast-non-reasoning"), (0.6, 2.4));
        assert_eq!(per_million("gpt-4o"), (2.5, 10.0));
    }

    // "non-reasoning" contains "reasoning", so a naive substring order billed
    // the cheapest Grok tier at the fast-reasoning rate.
    #[test]
    fn non_reasoning_grok_is_not_billed_as_reasoning() {
        let t3 = per_million("grok-4-1-fast-non-reasoning");
        let t2 = per_million("grok-4-1-fast-reasoning");
        assert_eq!(t3, (0.6, 2.4));
        assert_ne!(t3, t2, "tier 3 must not bill at the tier 2 rate");
    }

    #[test]
    fn an_unknown_model_falls_back_rather_than_billing_zero() {
        // Zero would hide spend; the fallback is deliberately non-zero.
        assert!(compute_cost("some-new-model", 1_000_000, 0) > 0.0);
    }

    #[test]
    fn negative_and_zero_token_counts_are_safe() {
        assert_eq!(
            compute_cost(crate::constants::MODEL_DEEPSEEK_V4_FLASH, 0, 0),
            0.0
        );
        assert_eq!(
            compute_cost_cached(crate::constants::MODEL_DEEPSEEK_V4_FLASH, -5, -5, -5),
            0.0
        );
    }

    #[test]
    fn providers_without_a_cache_rate_bill_cached_input_in_full() {
        let with = compute_cost_cached("claude-sonnet-4-20250514", 1_000_000, 1_000_000, 0);
        let without = compute_cost("claude-sonnet-4-20250514", 1_000_000, 0);
        assert!((with - without).abs() < 1e-6, "{with} vs {without}");
    }
}
