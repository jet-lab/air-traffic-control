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

use actix_web::web::{Bytes, Data};
use actix_web::{middleware, post, App, HttpResponse, HttpServer, Responder};
use rand::distributions::{Distribution, Standard};
use rand::{thread_rng, Rng};
use std::env::var;

const SUCCESS_PERCENTAGE: f32 = 0.75;

struct ServiceConfig {
    rpc_endpoint: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let port: u16 = var("PORT").map(|p| p.parse().unwrap()).unwrap_or(8080);

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .app_data(Data::new(ServiceConfig {
                rpc_endpoint: var("RPC_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:8899".into()),
            }))
            .service(rpc)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

#[post("/")]
async fn rpc(
    payload: Bytes,
    data: Data<ServiceConfig>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    dbg!(&payload);

    if thread_rng().gen::<f32>() >= SUCCESS_PERCENTAGE {
        return Ok(RpcEvent::random().respond().await);
    }

    let res = reqwest::Client::new()
        .post(data.rpc_endpoint.clone())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(payload)
        .send()
        .await?
        .text()
        .await?;

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(res))
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
enum RpcEvent {
    RateLimit,
    Timeout,
}

impl RpcEvent {
    pub fn random() -> Self {
        rand::random()
    }

    pub async fn respond(&self) -> HttpResponse {
        match self {
            RpcEvent::RateLimit => HttpResponse::TooManyRequests().finish(),
            RpcEvent::Timeout => {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                HttpResponse::RequestTimeout().finish()
            }
        }
    }
}

impl Distribution<RpcEvent> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RpcEvent {
        match rng.gen_range(0..=1) {
            0 => RpcEvent::RateLimit,
            _ => RpcEvent::Timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn event_responses() {
        assert_eq!(
            RpcEvent::RateLimit.respond().await.status(),
            HttpResponse::TooManyRequests().finish().status(),
        );

        assert_eq!(
            RpcEvent::Timeout.respond().await.status(),
            HttpResponse::RequestTimeout().finish().status(),
        );
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
