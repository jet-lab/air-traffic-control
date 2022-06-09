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

use actix_web::{web, HttpResponse};
use rand::distributions::{Distribution, Standard};
use rand::{thread_rng, Rng};
use serde_json::json;

use crate::GlobalState;

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum RpcEvent {
    FalsifiedSignature,
    RateLimit,
    Timeout,
}

impl RpcEvent {
    pub fn random() -> Self {
        rand::random()
    }

    pub async fn respond(&self, data: &web::Data<GlobalState>) -> HttpResponse {
        match self {
            RpcEvent::FalsifiedSignature => {
                let mut rng = thread_rng();
                let sig = generate_fake_signature(&mut rng);

                let mut fake_sigs = data.fake_signatures.lock().unwrap();
                dbg!(&fake_sigs);
                fake_sigs.push(sig.clone());

                HttpResponse::Ok().content_type("application/json").body(
                    json!({
                        "jsonrpc": "2.0",
                        "result": sig,
                        "id": 1,
                    })
                    .to_string(),
                )
            }
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
                .respond(&web::Data::new(GlobalState::default()))
                .await
                .status(),
            HttpResponse::TooManyRequests().finish().status(),
        );

        assert_eq!(
            RpcEvent::Timeout
                .respond(&web::Data::new(GlobalState::default()))
                .await
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
