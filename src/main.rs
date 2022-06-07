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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port: u16 = match option_env!("PORT") {
        Some(p) => p.parse().unwrap(),
        None => 8008,
    };

    HttpServer::new(|| App::new().service(rpc))
        .bind(("0.0.0.0", port))?
        .run()
        .await
}

#[post("/")]
async fn rpc(payload: Bytes) -> impl Responder {
    dbg!(payload);
    HttpResponse::Ok()
}
