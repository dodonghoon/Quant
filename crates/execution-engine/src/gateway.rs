use reqwest::{Client, header};
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;
use sha2::{Sha512, Digest};
use hex;
use log::{info, error, warn};

/// Upbit KRW market minimum order amount (KRW).
/// Orders below this threshold are rejected by the exchange with
/// `under_min_total_bid`. Discovered 2026-03-27 during live testing.
const MIN_ORDER_KRW: f64 = 5_000.0;
use urlencoding::decode;

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    pub market: String,
    pub side: String,
    pub volume: Option<String>,
    pub price: Option<String>,
    pub ord_type: String,
}

#[derive(Debug, Deserialize)]
pub struct OrderResponse {
    pub uuid: Option<String>,
    pub side: Option<String>,
    pub ord_type: Option<String>,
    pub price: Option<String>,
    pub state: Option<String>,
    pub market: Option<String>,
}

pub struct Gateway {
    client: Client,
    access_key: String,
    secret_key: String,
    base_url: String,
    trading_mode: String,
}

impl Gateway {
    pub fn new() -> Self {
        dotenvy::from_filename("config/.env.production").ok();
        Self {
            client: Client::new(),
            access_key: env::var("UPBIT_ACCESS_KEY").expect("UPBIT_ACCESS_KEY not set"),
            secret_key: env::var("UPBIT_SECRET_KEY").expect("UPBIT_SECRET_KEY not set"),
            base_url: env::var("UPBIT_BASE_URL")
                .unwrap_or_else(|_| "https://api.upbit.com".to_string()),
            trading_mode: env::var("TRADING_MODE").unwrap_or_else(|_| "paper".to_string()),
        }
    }

    fn generate_token(&self, query_string: Option<&str>) -> String {
        let mut claims = serde_json::json!({
            "access_key": self.access_key,
            "nonce": Uuid::new_v4().to_string(),
        });

        if let Some(qs) = query_string {
            let unquoted_qs = decode(qs).expect("UTF-8 string").into_owned();
            let mut hasher = Sha512::new();
            hasher.update(unquoted_qs.as_bytes());
            let query_hash = hex::encode(hasher.finalize());

            claims.as_object_mut().unwrap().insert(
                "query_hash".to_string(),
                serde_json::Value::String(query_hash),
            );
            claims.as_object_mut().unwrap().insert(
                "query_hash_alg".to_string(),
                serde_json::Value::String("SHA512".to_string()),
            );
        }

        let header = Header::new(Algorithm::HS512);
        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(self.secret_key.as_bytes()),
        )
        .expect("JWT encoding failed")
    }

    pub async fn send_order(
        &self,
        order: &Order,
    ) -> Result<OrderResponse, Box<dyn std::error::Error>> {
        // ── Pre-flight: Upbit minimum order validation ────────────────────────
        // `price` field = KRW amount for ord_type="price" (market buy by amount)
        // `volume` field = coin qty for ord_type="market"/"limit"
        // We validate the KRW-equivalent estimate for the `price` ord_type.
        if order.ord_type == "price" {
            if let Some(ref price_str) = order.price {
                if let Ok(krw) = price_str.parse::<f64>() {
                    if krw < MIN_ORDER_KRW {
                        let msg = format!(
                            "Order rejected before sending: {:.0} KRW < minimum {:.0} KRW \
                             (Upbit under_min_total_bid). Increase order size.",
                            krw, MIN_ORDER_KRW
                        );
                        warn!("[GATEWAY] {}", msg);
                        return Err(msg.into());
                    }
                }
            }
        }

        if self.trading_mode == "paper" {
            info!("[PAPER MODE] Order simulated: {:?}", order);
            return Ok(OrderResponse {
                uuid: Some(Uuid::new_v4().to_string()),
                side: Some(order.side.clone()),
                ord_type: Some(order.ord_type.clone()),
                price: order.price.clone(),
                state: Some("done".to_string()),
                market: Some(order.market.clone()),
            });
        }

        // Rate-limit: max 8 req/sec for orders
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let query_string = serde_urlencoded::to_string(order)?;
        let token = self.generate_token(Some(&query_string));

        let res = self
            .client
            .post(format!("{}/v1/orders", self.base_url))
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(order)
            .send()
            .await?;

        if res.status().is_success() {
            let parsed: OrderResponse = res.json().await?;
            Ok(parsed)
        } else {
            let status = res.status();
            let err_text = res.text().await?;
            error!("Upbit API Error [{}]: {}", status, err_text);
            Err(err_text.into())
        }
    }
}
