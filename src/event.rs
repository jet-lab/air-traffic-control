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

use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{web, HttpResponse};
use rand::distributions::{Distribution, Standard};
use rand::{thread_rng, Rng};
use serde_json::json;
use std::str::FromStr;

use crate::service::{passthrough, GlobalState};

/// Enum declaraction to define and implement the logic
/// for various types of Solana RPC and transaction failure
/// and error events from the nodes.
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum RpcEvent {
    FalsifiedSignature,
    Latency,
    RateLimit,
    Timeout,
    UnconfirmedSignature,
}

impl RpcEvent {
    /// Returns a randomly selected `RpcEvent` variant.
    ///
    /// The variants that are used in the random distribution are:
    /// - `RpcEvent::Latency`
    /// - `RpcEvent::RateLimit`
    /// - `RpcEvent::Timeout`
    ///
    /// The remaining variants are utilized in special cases that require more
    /// complex RPC call interception and spoofing.
    ///
    /// # Example
    ///
    /// ```
    /// use atc::event::RpcEvent;
    ///
    /// let event: RpcEvent = RpcEvent::random();
    /// ```
    ///
    /// Since `RpcEvent` implements `rand::Distribution<RpcEvent>`, you can also just use the
    /// native `rand::random` function to generate a random variant.
    ///
    /// ```ignore
    /// let event: RpcEvent = rand::random();
    /// ```
    pub fn random() -> Self {
        rand::random()
    }

    /// The `HttpResponse` responder for each variant of `RpcEvent` to define how
    /// they should interact with the incoming request and shared application data
    /// and defines the event type's RPC or transaction interception behavior prior
    /// to responding to the incoming request itself.
    pub async fn respond(
        &self,
        payload: &web::Bytes,
        data: &web::Data<GlobalState>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error>> {
        let mut rng = thread_rng();

        match self {
            RpcEvent::FalsifiedSignature => {
                let sig = generate_fake_signature(&mut rng);

                let mut fake_sigs = data.fake_signatures.write().unwrap();
                dbg!(&fake_sigs);
                fake_sigs.push(sig.clone());

                Ok(HttpResponse::Ok()
                    .insert_header(("X-ATC-Event", "FalsifiedSignature"))
                    .content_type("application/json")
                    .body(
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
                let mut res = passthrough(payload, data).await?;
                res.headers_mut().insert(
                    HeaderName::from_str("X-ATC-Event")?,
                    HeaderValue::from_str("Latency")?,
                );
                Ok(res)
            }
            RpcEvent::RateLimit => Ok(HttpResponse::TooManyRequests()
                .insert_header(("X-ATC-Event", "RateLimit"))
                .finish()),
            RpcEvent::Timeout => {
                tokio::time::sleep(tokio::time::Duration::from_secs(rng.gen_range(15..=20))).await;
                Ok(HttpResponse::RequestTimeout()
                    .insert_header(("X-ATC-Event", "Timeout"))
                    .finish())
            }
            RpcEvent::UnconfirmedSignature => {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                Ok(HttpResponse::Ok()
                    .insert_header(("X-ATC-Event", "UnconfirmedSignature"))
                    .content_type("application/json")
                    .body(
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
        match rng.gen_range(0..=2) {
            0 => RpcEvent::Latency,
            1 => RpcEvent::RateLimit,
            _ => RpcEvent::Timeout,
        }
    }
}

/// Creates a randomly generated 64-byte and base-58 encoded signature string
/// to be used for mocking transaction confirmation errors during intercepted RPC calls.
///
/// # Example
/// ```ignore
/// let mut rng = rand::thread_rng();
/// let sig = generate_fake_signature(&mut rng);
/// ```
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
        let rate_limit_res = RpcEvent::RateLimit
            .respond(
                &web::Bytes::default(),
                &web::Data::new(GlobalState::default()),
            )
            .await
            .unwrap();

        assert_eq!(
            rate_limit_res.status(),
            HttpResponse::TooManyRequests().finish().status(),
        );
        assert_eq!(
            rate_limit_res.headers().get("X-ATC-Event"),
            Some(&HeaderValue::from_str("RateLimit").unwrap())
        );

        let timeout_res = RpcEvent::Timeout
            .respond(
                &web::Bytes::default(),
                &web::Data::new(GlobalState::default()),
            )
            .await
            .unwrap();

        assert_eq!(
            timeout_res.status(),
            HttpResponse::RequestTimeout().finish().status(),
        );
        assert_eq!(
            timeout_res.headers().get("X-ATC-Event"),
            Some(&HeaderValue::from_str("Timeout").unwrap())
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
