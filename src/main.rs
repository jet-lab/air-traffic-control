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

use actix_web::{middleware, post, App, HttpResponse, HttpServer};
use actix_web::{web, HttpResponseBuilder};
use rand::distributions::{Distribution, Standard};
use rand::{thread_rng, Rng};
use serde_json::json;
use std::env::var;
use std::sync::Mutex;

const SUCCESS_RPC_PERCENTAGE: f32 = 1.0;
const SUCCESS_TX_PERCENTAGE: f32 = 0.25;

#[derive(Default)]
pub struct GlobalState {
    fake_signatures: Mutex<Vec<String>>,
    rpc_endpoint: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let workers: usize = var("ATC_WORKERS").map(|w| w.parse().unwrap()).unwrap_or(10);
    let port: u16 = var("ATC_PORT").map(|p| p.parse().unwrap()).unwrap_or(8080);

    let shared_data = web::Data::new(GlobalState {
        fake_signatures: Mutex::new(Vec::new()),
        rpc_endpoint: var("SOLANA_RPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:8899".into()),
    });

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .app_data(shared_data.clone())
            .service(rpc)
    })
    .bind(("0.0.0.0", port))?
    .workers(workers)
    .run()
    .await
}

#[post("/")]
async fn rpc(
    payload: web::Bytes,
    data: web::Data<GlobalState>,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    dbg!(&payload);

    let req: serde_json::Value = serde_json::from_slice(payload.as_ref())?;
    let method = req.get("method").unwrap().as_str().unwrap();

    let mut rng = thread_rng();

    if rng.gen::<f32>() >= SUCCESS_RPC_PERCENTAGE {
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

            if data.fake_signatures.lock().unwrap().contains(&param_sig) {
                RpcEvent::UnconfirmedSignature
                    .respond(&payload, &data)
                    .await
            } else {
                passthrough(&payload, &data).await
            }
        }
        "sendTransaction" if rng.gen::<f32>() >= SUCCESS_TX_PERCENTAGE => {
            RpcEvent::FalsifiedSignature.respond(&payload, &data).await
        }
        _ => passthrough(&payload, &data).await,
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
enum RpcEvent {
    FalsifiedSignature,
    Latency,
    RateLimit,
    Timeout,
    UnconfirmedSignature,
}

impl RpcEvent {
    pub fn random() -> Self {
        rand::random()
    }

    pub async fn respond(
        &self,
        payload: &web::Bytes,
        data: &web::Data<GlobalState>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error>> {
        let mut rng = thread_rng();

        match self {
            RpcEvent::FalsifiedSignature => {
                let sig = generate_fake_signature(&mut rng);

                let mut fake_sigs = data.fake_signatures.lock().unwrap();
                dbg!(&fake_sigs);
                fake_sigs.push(sig.clone());

                Ok(HttpResponse::Ok().content_type("application/json").body(
                    json!({
                        "jsonrpc": "2.0",
                        "result": sig,
                        "id": 1,
                    })
                    .to_string(),
                ))
            }
            RpcEvent::Latency => {
                tokio::time::sleep(tokio::time::Duration::from_secs(rng.gen_range(5..=10))).await;
                passthrough(payload, data).await
            }
            RpcEvent::RateLimit => Ok(HttpResponse::TooManyRequests().finish()),
            RpcEvent::Timeout => {
                tokio::time::sleep(tokio::time::Duration::from_secs(rng.gen_range(15..=20))).await;
                Ok(HttpResponse::RequestTimeout().finish())
            }
            RpcEvent::UnconfirmedSignature => {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                Ok(HttpResponse::Ok().content_type("application/json").body(
                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "context": {
                                "apiVersion": "1.10.24",
                                "slot": 1
                            },
                            "value": [null]
                        },
                        "id": 1
                    })
                    .to_string(),
                ))
            }
        }
    }
}

impl Distribution<RpcEvent> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RpcEvent {
        match rng.gen_range(0..=1) {
            0 => RpcEvent::RateLimit,
            1 => RpcEvent::Latency,
            _ => RpcEvent::Timeout,
        }
    }
}

async fn passthrough(
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

fn generate_fake_signature<R: Rng + ?Sized>(r: &mut R) -> String {
    bs58::encode(
        r.sample_iter(&rand::distributions::Alphanumeric)
            .take(64)
            .map(char::from)
            .collect::<String>(),
    )
    .into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn event_responses() {
        assert_eq!(
            RpcEvent::RateLimit
                .respond(
                    &web::Bytes::default(),
                    &web::Data::new(GlobalState::default())
                )
                .await
                .unwrap()
                .status(),
            HttpResponse::TooManyRequests().finish().status(),
        );

        assert_eq!(
            RpcEvent::Timeout
                .respond(
                    &web::Bytes::default(),
                    &web::Data::new(GlobalState::default())
                )
                .await
                .unwrap()
                .status(),
            HttpResponse::RequestTimeout().finish().status(),
        );
    }

    #[test]
    fn fake_signature() {
        let mut r = thread_rng();
        let sig1 = generate_fake_signature(&mut r);
        let sig2 = generate_fake_signature(&mut r);

        assert_ne!(sig1, sig2);
        assert_eq!(bs58::decode(sig1).into_vec().unwrap().len(), 64);
        assert_eq!(bs58::decode(sig2).into_vec().unwrap().len(), 64);
    }

    #[test]
    fn random_events() {
        let events1: Vec<RpcEvent> = (0..10).map(|_| RpcEvent::random()).collect();
        assert!(!events1.is_empty());

        let events2: Vec<RpcEvent> = (0..10).map(|_| RpcEvent::random()).collect();
        assert!(!events2.is_empty());

        assert_ne!(events1, events2);
    }
}
