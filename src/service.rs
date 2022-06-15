// Copyright (C) 2022 JET PROTOCOL HOLDINGS, LLC.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use actix_web::{get, post, web, HttpResponse, HttpResponseBuilder};
use rand::{thread_rng, Rng};
use std::sync::RwLock;

use crate::config::{Config, PercentageSettings};
use crate::event::RpcEvent;

/// The shared global application state to be used for internal
/// proxy service tracking of RPC event interception details
/// and external targets.
#[derive(Default)]
pub struct GlobalState {
    pub fake_signatures: RwLock<Vec<String>>,
    pub percentages: PercentageSettings,
    pub rpc_endpoint: String,
}

impl From<&Config> for GlobalState {
    fn from(c: &Config) -> Self {
        Self {
            fake_signatures: RwLock::new(Vec::new()),
            percentages: c.settings.percentages.clone(),
            rpc_endpoint: c.settings.rpc_endpoint.clone(),
        }
    }
}

/// HTTP responder function to perform a simple request passthrough
/// to the validator that the proxy is fronting to get an non-manipulated
/// RPC method reponse to the incoming or constructed request.
pub async fn passthrough(
    payload: &web::Bytes,
    data: &web::Data<GlobalState>,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    let res = reqwest::Client::new()
        .post(data.rpc_endpoint.clone())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(payload.clone())
        .send()
        .await?;

    Ok(HttpResponseBuilder::new(res.status()).body(res.text().await?))
}

#[get("/health")]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[post("/")]
pub async fn rpc(
    payload: web::Bytes,
    data: web::Data<GlobalState>,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    dbg!(&payload);

    let req: serde_json::Value = serde_json::from_slice(payload.as_ref())?;
    let method = req.get("method").unwrap().as_str().unwrap();

    let mut rng = thread_rng();

    if rng.gen::<f32>() >= data.percentages.rpc_success {
        return RpcEvent::random().respond(&payload, &data).await;
    }

    match method {
        "getSignatureStatuses" => {
            let param_sig = req.get("params").unwrap().as_array().unwrap()[0]
                .as_array()
                .unwrap()[0]
                .as_str()
                .unwrap()
                .to_string();

            if data.fake_signatures.read().unwrap().contains(&param_sig) {
                RpcEvent::UnconfirmedSignature
                    .respond(&payload, &data)
                    .await
            } else {
                passthrough(&payload, &data).await
            }
        }
        "sendTransaction" if rng.gen::<f32>() >= data.percentages.tx_success => {
            RpcEvent::FalsifiedSignature.respond(&payload, &data).await
        }
        _ => passthrough(&payload, &data).await,
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::header::{ContentType, HeaderValue};
    use actix_web::http::StatusCode;
    use actix_web::{test, web, App};
    use serde_json::{json, Value};

    use super::*;
    use crate::config::PercentageSettings;

    #[actix_web::test]
    async fn health_ok() {
        let app = test::init_service(App::new().service(health)).await;
        let res =
            test::call_service(&app, test::TestRequest::get().uri("/health").to_request()).await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn fake_signature_received() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(GlobalState {
                    fake_signatures: RwLock::new(Vec::new()),
                    percentages: PercentageSettings {
                        rpc_success: 1.0,
                        tx_success: 0.0,
                    },
                    rpc_endpoint: "".into(),
                }))
                .service(rpc),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .insert_header(ContentType::json())
            .set_payload(
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "sendTransaction",
                    "params": [""]
                })
                .to_string(),
            )
            .to_request();

        let res = test::call_service(&app, req).await;

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get("X-ATC-Event"),
            Some(&HeaderValue::from_str("FalsifiedSignature").unwrap())
        );

        let body: Value = test::read_body_json(res).await;
        let sig = body.get("result").unwrap().as_str().unwrap();

        assert_eq!(bs58::decode(sig).into_vec().unwrap().len(), 64);
    }

    #[actix_web::test]
    async fn unconfirmed_fake_signature() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(GlobalState {
                    fake_signatures: RwLock::new(Vec::new()),
                    percentages: PercentageSettings {
                        rpc_success: 1.0,
                        tx_success: 0.0,
                    },
                    rpc_endpoint: "".into(),
                }))
                .service(rpc),
        )
        .await;

        let tx_req = test::TestRequest::post()
            .uri("/")
            .insert_header(ContentType::json())
            .set_payload(
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "sendTransaction",
                    "params": [""]
                })
                .to_string(),
            )
            .to_request();

        let tx_res: Value = test::call_and_read_body_json(&app, tx_req).await;

        let cnf_req = test::TestRequest::post()
            .uri("/")
            .insert_header(ContentType::json())
            .set_payload(
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getSignatureStatuses",
                    "params": [
                        [tx_res.get("result").unwrap().as_str()]
                    ]
                })
                .to_string(),
            )
            .to_request();

        let cnf_res = test::call_service(&app, cnf_req).await;

        assert_eq!(
            cnf_res.headers().get("X-ATC-Event"),
            Some(&HeaderValue::from_str("UnconfirmedSignature").unwrap())
        );

        let cnf_body: Value = test::read_body_json(cnf_res).await;

        assert_eq!(
            *cnf_body.get("result").unwrap().get("value").unwrap(),
            json!([null])
        );
    }
}
