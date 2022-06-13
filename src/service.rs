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
use std::sync::Mutex;

use crate::config::PercentageSettings;
use crate::event::RpcEvent;

#[derive(Default)]
pub struct GlobalState {
    pub fake_signatures: Mutex<Vec<String>>,
    pub percentages: PercentageSettings,
    pub rpc_endpoint: String,
}

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

            if data.fake_signatures.lock().unwrap().contains(&param_sig) {
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
