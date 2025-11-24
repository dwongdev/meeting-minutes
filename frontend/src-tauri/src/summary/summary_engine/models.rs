// Model definitions and prompt templates for built-in AI summary generation
// Designed for easy extension - just add new entries to get_available_models()

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// Model Definitions
// ============================================================================

/// Sampling parameters for text generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingParams {
    /// Temperature - controls randomness (0.0 = deterministic, 1.0 = balanced, 2.0 = very creative)
    pub temperature: f32,

    /// Top-K sampling - limits vocabulary to top K tokens (0 = disabled)
    pub top_k: i32,

    /// Top-P (nucleus) sampling - cumulative probability threshold (1.0 = disabled)
    pub top_p: f32,

    /// Stop tokens - generation stops when any of these appear in output
    pub stop_tokens: Vec<String>,
}

/// Definition of a built-in AI model with all metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDef {
    /// Model name in format "family:variant" (e.g., "gemma3:1b")
    /// This is what's stored in database as model field when provider="builtin-ai"
    pub name: String,

    /// Display name for UI (e.g., "Gemma 3 1B (Fast)")
    pub display_name: String,

    /// GGUF filename on disk (e.g., "gemma-3-1b-it-q4_0.gguf")
    pub gguf_file: String,

    /// Template name for prompt formatting (e.g., "gemma3")
    pub template: String,

    /// Download URL (HuggingFace or other source)
    pub download_url: String,

    /// File size in MB
    pub size_mb: u64,

    /// Context window size in tokens (configurable per model!)
    /// This is used for chunking in processor.rs
    pub context_size: u32,

    /// Model layer count (for GPU offloading calculation)
    pub layer_count: u32,

    /// Sampling parameters for this model
    pub sampling: SamplingParams,

    /// Short description for UI
    pub description: String,
}

/// Get all available built-in AI models
/// Add new models here - the system will automatically detect and manage them
pub fn get_available_models() -> Vec<ModelDef> {
    vec![
        // Gemma 3 1B - Fast tier
        ModelDef {
            name: "gemma3:1b".to_string(),
            display_name: "Gemma 3 1B (Fast)".to_string(),
            gguf_file: "gemma-3-1b-it-Q4_K_M.gguf".to_string(),
            template: "gemma3".to_string(),
            download_url: "https://huggingface.co/unsloth/gemma-3-1b-it-GGUF/resolve/main/gemma-3-1b-it-Q4_K_M.gguf".to_string(),
            size_mb: 806,
            context_size: 8192,  // Gemma 3's native context window
            layer_count: 26,     // Gemma 3 1B has 26 transformer layers
            sampling: SamplingParams {
                temperature: 1.0,
                top_k: 64,
                top_p: 0.95,
                stop_tokens: vec!["<end_of_turn>".to_string()],
            },
            description: "Fastest model. Runs on any hardware with ~1GB RAM. Good for quick summaries.".to_string(),
        },

        // Example: Add more models in the future
        // ModelDef {
        //     name: "qwen2.5:1.5b".to_string(),
        //     display_name: "Qwen 2.5 1.5B (Medium)".to_string(),
        //     gguf_file: "qwen2.5-1.5b-instruct-q4_0.gguf".to_string(),
        //     template: "chatml".to_string(),
        //     download_url: "https://huggingface.co/.../qwen2.5-1.5b-instruct-q4_0.gguf".to_string(),
        //     size_mb: 900,
        //     context_size: 8192,  // Qwen 2.5 has larger context!
        //     layer_count: 28,
        //     description: "Balanced model. Better quality, still fast. Requires ~1.5GB RAM.".to_string(),
        // },
    ]
}

/// Get a specific model by name
pub fn get_model_by_name(name: &str) -> Option<ModelDef> {
    get_available_models().into_iter().find(|m| m.name == name)
}

/// Get the default model (first in list)
pub fn get_default_model() -> ModelDef {
    get_available_models()
        .into_iter()
        .next()
        .expect("At least one model must be defined")
}

/// Resolve model name to full file path in the models directory
pub fn get_model_path(app_data_dir: &PathBuf, model_name: &str) -> Result<PathBuf> {
    let model = get_model_by_name(model_name)
        .ok_or_else(|| anyhow!("Unknown model: {}", model_name))?;

    let models_dir = get_models_directory(app_data_dir);
    let model_path = models_dir.join(&model.gguf_file);

    Ok(model_path)
}

/// Get the models directory path for built-in AI
pub fn get_models_directory(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("models").join("summary")
}

// ============================================================================
// Prompt Templates (Model-Specific Formatting)
// ============================================================================

/// Gemma 3 chat template format
pub const GEMMA3_TEMPLATE: &str = "\
<start_of_turn>user
{system_prompt}<end_of_turn>
<start_of_turn>user
{user_prompt}<end_of_turn>
<start_of_turn>model
";

/// ChatML template format (used by Qwen, Phi, and many others)
/// Format: <|im_start|>system\n{system}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>...
pub const CHATML_TEMPLATE: &str = "<|im_start|>system\n{system_prompt}<|im_end|>\n<|im_start|>user\n{user_prompt}<|im_end|>\n<|im_start|>assistant\n";

/// Llama 3 chat template format
/// Format: <|begin_of_text|><|start_header_id|>system<|end_header_id|>\n{system}<|eot_id|>...
pub const LLAMA3_TEMPLATE: &str = "<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n{system_prompt}<|eot_id|><|start_header_id|>user<|end_header_id|>\n{user_prompt}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n";

/// Format a prompt using the specified template
///
/// # Arguments
/// * `template_name` - Template identifier (e.g., "gemma3", "chatml", "llama3")
/// * `system_prompt` - System message (instructions for the model)
/// * `user_prompt` - User message (actual task/question)
///
/// # Returns
/// Formatted prompt string ready to send to llama-helper
pub fn format_prompt(
    template_name: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String> {
    let template = match template_name {
        "gemma3" => GEMMA3_TEMPLATE,
        "chatml" => CHATML_TEMPLATE,
        "llama3" => LLAMA3_TEMPLATE,
        _ => return Err(anyhow!("Unknown template: {}", template_name)),
    };

    let formatted = template
        .replace("{system_prompt}", system_prompt)
        .replace("{user_prompt}", user_prompt);

    Ok(formatted)
}

// ============================================================================
// Configuration Constants
// ============================================================================

/// Default max tokens for generation (increased for better summary quality)
pub const DEFAULT_MAX_TOKENS: i32 = 2048;

/// Idle timeout for sidecar (seconds) - can be overridden via LLAMA_IDLE_TIMEOUT env var
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// Generation timeout (how long to wait for a response)
pub const GENERATION_TIMEOUT_SECS: u64 = 120; // 2 minutes

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_available_models() {
        let models = get_available_models();
        assert!(!models.is_empty(), "At least one model must be defined");
        assert!(models.iter().any(|m| m.name == "gemma3:1b"));
    }

    #[test]
    fn test_get_model_by_name() {
        let model = get_model_by_name("gemma3:1b");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.context_size, 2048);
        assert_eq!(model.layer_count, 26);
        // Verify sampling parameters
        assert_eq!(model.sampling.temperature, 1.0);
        assert_eq!(model.sampling.top_k, 64);
        assert_eq!(model.sampling.top_p, 0.95);
        assert_eq!(model.sampling.stop_tokens, vec!["<end_of_turn>"]);
    }

    #[test]
    fn test_format_prompt_gemma3() {
        let system = "You are a helpful assistant.";
        let user = "Summarize this meeting.";

        let prompt = format_prompt("gemma3", system, user).unwrap();
        assert!(prompt.contains("<start_of_turn>user"));
        assert!(prompt.contains("<end_of_turn>"));
        assert!(prompt.contains("<start_of_turn>model"));
        assert!(prompt.contains(system));
        assert!(prompt.contains(user));
    }

    #[test]
    fn test_format_prompt_invalid() {
        let result = format_prompt("invalid", "sys", "user");
        assert!(result.is_err());
    }

    #[test]
    fn test_model_path_resolution() {
        use std::path::PathBuf;
        let app_data = PathBuf::from("/test/appdata");
        let path = get_model_path(&app_data, "gemma3:1b").unwrap();
        assert!(path.to_string_lossy().contains("models/summary"));
        assert!(path.to_string_lossy().contains("gemma-3-1b-it-q4_0.gguf"));
    }
}
