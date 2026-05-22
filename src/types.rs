use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};

pub const MODEL_ID: &str = "protectai/deberta-v3-base-prompt-injection-v2";

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckResult {
    pub label: String,
    pub is_injection: bool,
    /// Rejected without model inference (e.g. non-English).
    pub rejected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    pub inferred: bool,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    pub name: String,
    pub version: String,
    pub model: String,
}

pub fn version_info() -> VersionInfo {
    VersionInfo {
        name: "trypanophobe".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        model: MODEL_ID.into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

impl CheckResult {
    pub fn rejected_non_english() -> Self {
        Self {
            label: "REJECTED".into(),
            is_injection: false,
            rejected: true,
            score: None,
            inferred: false,
            language: "non_english".into(),
        }
    }

    pub fn from_model(label: &str, is_injection: bool, score: f32) -> Self {
        Self {
            label: label.to_string(),
            is_injection,
            rejected: false,
            score: Some(score),
            inferred: true,
            language: "en".into(),
        }
    }

    pub fn http_status(&self) -> salvo::http::StatusCode {
        use salvo::http::StatusCode;
        if self.rejected {
            StatusCode::BAD_REQUEST
        } else if self.is_injection {
            StatusCode::NOT_ACCEPTABLE
        } else {
            StatusCode::ACCEPTED
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use salvo::http::StatusCode;

    #[test]
    fn rejected_non_english_shape() {
        let r = CheckResult::rejected_non_english();
        assert!(r.rejected);
        assert!(!r.is_injection);
        assert_eq!(r.label, "REJECTED");
        assert_eq!(r.http_status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_status_variants() {
        assert_eq!(
            CheckResult::from_model("SAFE", false, 0.9).http_status(),
            StatusCode::ACCEPTED
        );
        assert_eq!(
            CheckResult::from_model("INJECTION", true, 0.9).http_status(),
            StatusCode::NOT_ACCEPTABLE
        );
    }

    #[test]
    fn version_info_fields() {
        let v = version_info();
        assert_eq!(v.name, "trypanophobe");
        assert_eq!(v.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(v.model, MODEL_ID);
    }

    #[test]
    fn check_result_serializes_score() {
        let safe = CheckResult::from_model("SAFE", false, 0.95);
        let json = serde_json::to_string(&safe).unwrap();
        assert!(json.contains("\"score\":0.95"));
        let rej = CheckResult::rejected_non_english();
        let json = serde_json::to_string(&rej).unwrap();
        assert!(!json.contains("score"));
    }
}
