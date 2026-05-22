use crate::detector::Detector;
use crate::language::is_english;
use crate::inputs::{collect_check_items, CheckItem};
use crate::types::CheckResult;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckBatchOutcome {
    Ok,
    Failed,
}

pub fn run_check_batch(args: &[String]) -> Result<CheckBatchOutcome> {
    let items = collect_check_items(args)?;
    let mut detector: Option<Detector> = None;
    let mut failed = false;

    for item in &items {
        let result = check_one_prompt(item, &mut detector)?;
        let status = if result.rejected {
            "rejected"
        } else if result.is_injection {
            "injection"
        } else {
            "ok"
        };
        tracing::info!(target = %item.name, %status, "check result");

        if result.rejected || result.is_injection {
            failed = true;
        }
    }

    Ok(if failed {
        CheckBatchOutcome::Failed
    } else {
        CheckBatchOutcome::Ok
    })
}

pub fn check_one_prompt(item: &CheckItem, detector: &mut Option<Detector>) -> Result<CheckResult> {
    if !is_english(&item.text) {
        return Ok(CheckResult::rejected_non_english());
    }
    if detector.is_none() {
        *detector = Some(Detector::new()?);
    }
    detector.as_mut().unwrap().check(&item.text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::CheckItem;

    #[test]
    fn rejects_non_english_without_loading_model() {
        let item = CheckItem {
            name: "literal:1".into(),
            text: "Bonjour le monde".into(),
        };
        let mut detector = None;
        let result = check_one_prompt(&item, &mut detector).unwrap();
        assert!(result.rejected);
        assert!(detector.is_none());
    }

    #[test]
    fn batch_marks_failure_on_rejection() {
        let out = run_check_batch(&["Bonjour le monde".into()]).unwrap();
        assert_eq!(out, CheckBatchOutcome::Failed);
    }
}
