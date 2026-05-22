use crate::assets::load_model_assets;
use crate::language::is_english;
use crate::types::CheckResult;
use anyhow::{Context, Result};
use ndarray::Array2;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor;
use tokenizers::tokenizer::{TruncationParams, TruncationStrategy};
use tokenizers::Tokenizer;

pub struct Detector {
    tokenizer: Tokenizer,
    session: Session,
}

impl Detector {
    pub fn new() -> Result<Self> {
        tracing::info!("loading model into memory (decompressing embedded assets)…");

        let (model_bytes, tokenizer_bytes) = load_model_assets()?;
        tracing::info!("initializing tokenizer and ONNX session…");

        let mut tokenizer = Tokenizer::from_bytes(&tokenizer_bytes)
            .map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: 512,
                strategy: TruncationStrategy::LongestFirst,
                ..Default::default()
            }))
            .map_err(|e| anyhow::anyhow!("tokenizer truncation: {e}"))?;

        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("session builder: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("optimization level: {e}"))?
            .commit_from_memory(&model_bytes)
            .map_err(|e| anyhow::anyhow!("load onnx session: {e}"))?;

        tracing::info!("model ready");

        Ok(Self { tokenizer, session })
    }

    pub fn check(&mut self, text: &str) -> Result<CheckResult> {
        if !is_english(text) {
            return Ok(CheckResult::rejected_non_english());
        }

        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("tokenize: {e}"))?;

        let seq_len = encoding.get_ids().len();
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();

        let input_ids = Array2::from_shape_vec((1, seq_len), input_ids)?;
        let attention_mask = Array2::from_shape_vec((1, seq_len), attention_mask)?;

        let outputs = self.session.run(ort::inputs![
            "input_ids" => Tensor::from_array(input_ids)?,
            "attention_mask" => Tensor::from_array(attention_mask)?,
        ])?;

        let logits_key = outputs
            .keys()
            .find(|k| k.contains("logit"))
            .or_else(|| outputs.keys().next())
            .expect("model output");

        let logits = outputs[logits_key]
            .try_extract_array::<f32>()
            .map_err(|e| anyhow::anyhow!("extract logits: {e}"))?;

        let logits: Vec<f32> = logits
            .view()
            .to_slice()
            .context("logits slice")?
            .to_vec();

        let probs = softmax(&logits);
        let (label_idx, score) = probs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, &s)| (i, s))
            .unwrap_or((0, 0.0));

        let (label, is_injection) = if label_idx == 1 {
            ("INJECTION", true)
        } else {
            ("SAFE", false)
        };

        Ok(CheckResult::from_model(label, is_injection, score))
    }
}

pub(crate) fn softmax(logits: &[f32]) -> Vec<f32> {
    if logits.is_empty() {
        return vec![];
    }
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exp.iter().sum();
    exp.iter().map(|x| x / sum).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn softmax_empty_and_normal() {
        assert!(softmax(&[]).is_empty());
        let p = softmax(&[1.0, 2.0, 3.0]);
        assert_eq!(p.len(), 3);
        let sum: f32 = p.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(p[2] > p[0]);
    }
}
