use crate::language::is_english;
use crate::model_slot::SharedDetector;
use crate::types::{version_info, CheckRequest, CheckResult, ErrorResponse, VersionInfo};
use salvo::http::StatusCode;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use salvo::writing::Redirect;

pub fn build_router(detector: SharedDetector) -> Router {
    Router::new()
        .hoop(affix_state::inject(detector))
        .push(
            Router::with_path("api")
                .push(Router::with_path("check").post(check))
                .push(Router::with_path("version").get(version)),
        )
}

#[endpoint(
    tags("api"),
    request_body = CheckRequest,
    responses(
        (status_code = 202, description = "No prompt injection detected", body = CheckResult),
        (status_code = 406, description = "Prompt injection detected", body = CheckResult),
        (status_code = 400, description = "Rejected (e.g. non-English) or invalid request", body = CheckResult),
    ),
)]
async fn check(depot: &mut Depot, body: JsonBody<CheckRequest>, res: &mut Response) {
    let text = body.text.trim();
    if text.is_empty() {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Json(ErrorResponse {
            error: "text must not be empty".into(),
        }));
        return;
    }

    if !is_english(text) {
        let result = CheckResult::rejected_non_english();
        res.status_code(result.http_status());
        res.render(Json(result));
        return;
    }

    let slot = depot.obtain::<SharedDetector>().unwrap().clone();
    let text = text.to_string();

    let result = match tokio::task::spawn_blocking(move || {
        slot.with_detector(|detector| detector.check(&text))
    })
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(ErrorResponse {
                error: e.to_string(),
            }));
            return;
        }
        Err(e) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(ErrorResponse {
                error: format!("task failed: {e}"),
            }));
            return;
        }
    };

    res.status_code(result.http_status());
    res.render(Json(result));
}

#[endpoint(tags("api"))]
fn version() -> Json<VersionInfo> {
    Json(version_info())
}

pub fn build_service(router: Router, cors_origins: Vec<String>) -> Service {
    let cors = build_cors(cors_origins);
    Service::new(router).hoop(cors)
}

fn build_cors(origins: Vec<String>) -> impl Handler {
    use salvo::http::Method;
    use salvo_cors::{Any, Cors};

    let mut cors = Cors::new()
        .allow_methods(vec![Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(vec!["content-type", "authorization"]);

    if origins.iter().any(|o| o == "*") {
        cors = cors.allow_origin(Any);
    } else {
        let refs: Vec<&str> = origins.iter().map(String::as_str).collect();
        cors = cors.allow_origin(refs);
    }

    cors.into_handler()
}

#[handler]
async fn root_redirect(res: &mut Response) {
    res.render(Redirect::other("/swagger-ui/"));
}

pub fn mount_openapi(api_router: Router) -> Router {
    let doc = OpenApi::new("trypanophobe", env!("CARGO_PKG_VERSION")).merge_router(&api_router);
    api_router
        .unshift(doc.into_router("/api-doc/openapi.json"))
        .unshift(
            SwaggerUi::new("/api-doc/openapi.json")
                .title("trypanophobe")
                .into_router("/swagger-ui"),
        )
        .push(Router::with_path("/").get(root_redirect))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_slot::DetectorSlot;
    use salvo::http::StatusCode;
    use salvo::test::{ResponseExt, TestClient};

    fn service(cors: Vec<&str>) -> salvo::Service {
        build_service(
            mount_openapi(build_router(DetectorSlot::new())),
            cors.into_iter().map(str::to_string).collect(),
        )
    }

    #[tokio::test]
    async fn root_redirects_to_swagger() {
        let mut res = TestClient::get("http://localhost/")
            .send(&service(vec!["*"]))
            .await;
        assert_eq!(res.status_code, Some(StatusCode::SEE_OTHER));
        let loc = res.headers().get("location").unwrap().to_str().unwrap();
        assert!(loc.contains("/swagger-ui"));
    }

    #[tokio::test]
    async fn version_endpoint() {
        let mut res = TestClient::get("http://localhost/api/version")
            .send(&service(vec!["*"]))
            .await;
        assert_eq!(res.status_code, Some(StatusCode::OK));
        let body: VersionInfo = res.take_json().await.unwrap();
        assert_eq!(body.name, "trypanophobe");
    }

    #[tokio::test]
    async fn check_empty_text_is_bad_request() {
        let mut res = TestClient::post("http://localhost/api/check")
            .json(&CheckRequest {
                text: "   ".into(),
            })
            .send(&service(vec!["*"]))
            .await;
        assert_eq!(res.status_code, Some(StatusCode::BAD_REQUEST));
        let body: ErrorResponse = res.take_json().await.unwrap();
        assert!(body.error.contains("empty"));
    }

    #[tokio::test]
    async fn check_non_english_is_rejected_without_model() {
        let mut res = TestClient::post("http://localhost/api/check")
            .json(&CheckRequest {
                text: "Bonjour le monde".into(),
            })
            .send(&service(vec!["*"]))
            .await;
        assert_eq!(res.status_code, Some(StatusCode::BAD_REQUEST));
        let body: CheckResult = res.take_json().await.unwrap();
        assert!(body.rejected);
        assert_eq!(body.label, "REJECTED");
    }

    #[tokio::test]
    async fn cors_allows_specific_origin() {
        let mut res = TestClient::options("http://localhost/api/version")
            .add_header("Origin", "https://app.example", true)
            .add_header("Access-Control-Request-Method", "GET", true)
            .send(&service(vec!["https://app.example"]))
            .await;
        assert!(
            matches!(
                res.status_code,
                Some(StatusCode::OK) | Some(StatusCode::NO_CONTENT)
            )
        );
        let allow = res
            .headers()
            .get("access-control-allow-origin")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(allow, "https://app.example");
    }
}
