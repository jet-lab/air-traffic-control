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
use rand::{thread_rng, Rng};
use std::env::var;
use std::sync::Mutex;

mod events;

use events::RpcEvent;

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

    let port: u16 = var("PORT").map(|p| p.parse().unwrap()).unwrap_or(8080);

    let shared_data = web::Data::new(GlobalState {
        fake_signatures: Mutex::new(Vec::new()),
        rpc_endpoint: var("RPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:8899".into()),
    });

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .app_data(shared_data.clone())
            .service(rpc)
    })
    .bind(("0.0.0.0", port))?
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
        return Ok(RpcEvent::random().respond(&data).await);
    }

    match method {
        "getSignatureStatuses" => {
            let param_sig = req.get("params").unwrap().as_array().unwrap()[0]
                .as_str()
                .unwrap()
                .to_string();

            if data.fake_signatures.lock().unwrap().contains(&param_sig) {
                Ok(RpcEvent::Timeout.respond(&data).await)
            } else {
                passthrough(payload, &data).await
            }
        }
        "sendTransaction" if rng.gen::<f32>() >= SUCCESS_TX_PERCENTAGE => {
            Ok(RpcEvent::FalsifiedSignature.respond(&data).await)
        }
        _ => passthrough(payload, &data).await,
    }
}

async fn passthrough(
    payload: web::Bytes,
    data: &web::Data<GlobalState>,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    let res = reqwest::Client::new()
        .post(data.rpc_endpoint.clone())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(payload)
        .send()
        .await?;

    Ok(HttpResponseBuilder::new(res.status()).body(res.text().await?))
}
