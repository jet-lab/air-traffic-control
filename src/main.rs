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

use actix_web::{middleware, post, App, HttpRequest, HttpResponse, HttpServer, Responder};
use rand::distributions::{Distribution, Standard};
use rand::{thread_rng, Rng};

const SUCCESS_PERCENTAGE: f32 = 0.75;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let port: u16 = option_env!("PORT")
        .map(|p| p.parse().unwrap())
        .unwrap_or(8080);

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(handler)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

#[post("/")]
async fn handler(req: HttpRequest) -> impl Responder {
    dbg!(req);

    if thread_rng().gen::<f32>() >= SUCCESS_PERCENTAGE {
        return RpcEvent::random().respond();
    }

    HttpResponse::Ok().finish()
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

    pub fn respond(&self) -> HttpResponse {
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
