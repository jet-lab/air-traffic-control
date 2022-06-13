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

use actix_web::web;
use actix_web::{middleware, App, HttpServer};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

mod config;
mod event;
mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = option_env!("ATC_CONFIG_PATH")
        .map(|path| PathBuf::from_str(path).unwrap())
        .map(|p| config::Config::try_from(p).unwrap())
        .unwrap_or_default();

    println!("{:#?}", config);

    let shared_data = web::Data::new(service::GlobalState {
        fake_signatures: Mutex::new(Vec::new()),
        percentages: config.settings.percentages,
        rpc_endpoint: config.settings.rpc_endpoint,
    });

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .app_data(shared_data.clone())
            .service(service::health)
            .service(service::rpc)
    })
    .bind(("0.0.0.0", config.settings.port))?
    .workers(config.settings.workers)
    .run()
    .await
}
