//! Model pricing for token cost computation.
//!
//! Centralized pricing rates for all supported LLM providers.
//! Local models (Ollama) are free — returns $0.00.

/// Rate pair in USD per 1M tokens.
struct Rates {
    input: f32,
    output: f32,
    /// Rate for the cached portion of the input, where the provider offers one.
    cached_input: f32,
}

/// DeepInfra's namespace prefix for the DeepSeek family.
///
/// Matched as a prefix rather than by the bare `deepseek` substring: Ollama
/// also serves DeepSeek builds locally (`deepseek-coder`) and those are free,
/// while anything under this namespace is a hosted, billed model.
const DEEPINFRA_DEEPSEEK_PREFIX: &str = "deepseek-ai/";

/// DeepInfra's namespace prefix for the Z.ai GLM family.
const DEEPINFRA_GLM_PREFIX: &str = "zai-org/";

/// Approximate cost computation per model ($/1M tokens).
///
/// Local models (Ollama) are free — returns $0.00.
pub fn compute_cost(model_id: &str, input_tokens: i64, output_tokens: i64) -> f32 {
    compute_cost_cached(model_id, input_tokens, 0, output_tokens)
}

/// Cost computation that credits the provider's prompt cache.
///
/// `cached_input_tokens` is a SUBSET of `input_tokens`, matching the
/// OpenAI-compatible reporting this codebase normalizes to — the uncached
/// portion is the difference, never the whole. Treating it as an addition
/// would bill the cached tokens twice.
pub fn compute_cost_cached(
    model_id: &str,
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
) -> f32 {
    let Some(rates) = rates_for(model_id) else {
        return 0.0;
    };

    let cached = cached_input_tokens.clamp(0, input_tokens.max(0));
    let uncached = (input_tokens - cached).max(0);

    let per_million = |tokens: i64, rate: f32| (tokens as f32 / 1_000_000.0) * rate;

    per_million(uncached, rates.input)
        + per_million(cached, rates.cached_input)
        + per_million(output_tokens.max(0), rates.output)
}

/// Rates for a model, or `None` when it costs nothing (locally hosted).
fn rates_for(model_id: &str) -> Option<Rates> {
    // Hosted DeepSeek. Checked before the local patterns below, which would
    // otherwise match on the `deepseek` substring and bill it as free.
    if model_id.starts_with(DEEPINFRA_DEEPSEEK_PREFIX) {
        return Some(Rates {
            input: 0.08,
            output: 0.18,
            cached_input: 0.016,
        });
    }

    // Hosted GLM (Z.ai). Not the active tier model, but priced so runs
    // made while it was active still cost out correctly.
    //
    // These are DeepInfra's promotional rates: 50% off the $0.15 / $0.50 /
    // $0.03 list price, in effect as of 2026-08-29. When the promotion ends
    // these double; the numbers below are the ones actually billed today, so
    // estimates track reality rather than a list price nobody is charged.
    if model_id.starts_with(DEEPINFRA_GLM_PREFIX) {
        return Some(Rates {
            input: 0.075,
            output: 0.25,
            cached_input: 0.015,
        });
    }

    if is_local_model(model_id) {
        return None;
    }

    let (input, output) = if model_id.contains("opus") {
        (15.0_f32, 75.0_f32)
    } else if model_id.contains("sonnet") {
        (3.0, 15.0)
    } else if model_id.contains("haiku") {
        (0.25, 1.25)
    } else if model_id.contains("grok-4-0709") {
        // xAI Grok T1 (orchestrator)
        (3.0, 12.0)
    } else if model_id.contains("grok") && model_id.contains("non-reasoning") {
        // xAI Grok T3. Checked before the reasoning branch below, because
        // "non-reasoning" contains "reasoning" — without this the cheapest
        // tier billed at the fast-reasoning rate, 3.3x over.
        (0.6, 2.4)
    } else if model_id.contains("grok") && model_id.contains("reasoning") {
        // xAI Grok T2 (fast reasoning)
        (2.0, 8.0)
    } else if model_id.contains("grok") {
        // xAI Grok T3 / generic Grok fallback
        (0.6, 2.4)
    } else if model_id.contains("gpt-4o") {
        (2.5, 10.0)
    } else if model_id.contains("gpt-4") {
        (30.0, 60.0)
    } else {
        (1.0, 3.0)
    };

    Some(Rates {
        input,
        output,
        // Providers without a published cache rate bill cached input in full.
        cached_input: input,
    })
}

/// Known locally-hosted model patterns — no API cost.
///
/// Substring matching is deliberate: Ollama model ids are free-form (they come
/// straight from `OLLAMA_MODEL`) and may be namespaced, tagged, or both —
/// `llama3`, `library/llama3`, `hf.co/org/qwen2:7b`.
///
/// The family name alone is *not* enough, because hosted providers serve the
/// same families: `meta-llama/Meta-Llama-3.1-405B-Instruct` and
/// `mistralai/Mistral-Small-24B` both contain a local pattern while costing
/// real money. So a namespaced id counts as local only under a namespace that
/// is itself local. The default direction is deliberate — an unrecognised id
/// falls through to a non-zero rate, because under-billing is silent and
/// over-billing is not.
fn is_local_model(model_id: &str) -> bool {
    const LOCAL_PATTERNS: [&str; 8] = [
        "llama",
        "mistral",
        "codellama",
        "gemma",
        "phi",
        "qwen",
        "deepseek",
        "vicuna",
    ];
    /// Namespaces Ollama ids actually carry. Anything else before a `/` is a
    /// hosting org (`meta-llama/`, `mistralai/`, `Qwen/`, `google/`).
    const LOCAL_NAMESPACES: [&str; 2] = ["library", "hf.co"];

    // Ids are free-form and inconsistently cased (`Qwen/Qwen3`, `meta-llama/`).
    let id = model_id.to_ascii_lowercase();

    if !LOCAL_PATTERNS.iter().any(|p| id.contains(p)) {
        return false;
    }
    match id.split_once('/') {
        // A bare family name (`llama3`, `deepseek-r1:7b`) is an Ollama id.
        None => true,
        // Namespaced: local only under a local namespace, or when the id
        // carries an Ollama-style `:tag`, which hosted ids do not use. That
        // keeps a custom Ollama namespace (`myorg/mistral:latest`) free while
        // billing `meta-llama/Meta-Llama-3.1-405B-Instruct`.
        Some((namespace, rest)) => LOCAL_NAMESPACES.contains(&namespace) || rest.contains(':'),
    }
}

#[cfg(test)]
mod tests;
