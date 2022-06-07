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

use actix_web::web::Bytes;
use actix_web::{post, App, HttpResponse, HttpServer, Responder};
use rand::distributions::{Distribution, Standard};
use rand::Rng;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _rpc_endpoint = option_env!("RPC_ENDPOINT").unwrap_or("http://localhost:8899");

    let port: u16 = option_env!("PORT")
        .map(|p| p.parse().unwrap())
        .unwrap_or(8080);

    dbg!(port);

    HttpServer::new(|| App::new().service(rpc))
        .bind(("0.0.0.0", port))?
        .run()
        .await
}

#[post("/")]
async fn rpc(payload: Bytes) -> impl Responder {
    dbg!(payload);
    dbg!(RpcEvent::random());

    HttpResponse::Ok()
}

#[derive(Debug)]
pub enum RpcEvent {
    RateLimit,
    Timeout,
}

impl RpcEvent {
    pub fn random() -> Self {
        rand::random()
    }

    pub fn respond(&self) -> impl Responder {
        match self {
            RpcEvent::RateLimit => HttpResponse::TooManyRequests().finish(),
            RpcEvent::Timeout => HttpResponse::RequestTimeout().finish(),
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
