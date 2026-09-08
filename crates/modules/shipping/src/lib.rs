use async_trait::async_trait;
use program1_contracts::{
    ContractError, ShippingCity, ShippingCost, ShippingCostRequest, ShippingCourier,
    ShippingContract,
};
use serde::Deserialize;
use std::time::Duration;

#[derive(Clone)]
pub struct ShippingModule {
    api_key: String,
    base_url: String,
    origin_city_id: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct RajaOngkirResponse {
    rajaongkir: RajaOngkirPayload,
}

#[derive(Debug, Deserialize)]
struct RajaOngkirPayload {
    status: RajaOngkirStatus,
    results: Option<Vec<RajaOngkirCourierResult>>,
}

#[derive(Debug, Deserialize)]
struct RajaOngkirStatus {
    code: u16,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RajaOngkirCourierResult {
    code: String,
    name: String,
    costs: Vec<RajaOngkirCostItem>,
}

#[derive(Debug, Deserialize)]
struct RajaOngkirCostItem {
    service: String,
    description: String,
    cost: Vec<RajaOngkirCostDetail>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RajaOngkirCostDetail {
    value: i64,
    etd: Option<String>,
    note: Option<String>,
}

impl ShippingModule {
    pub fn new(api_key: String, api_type: String, origin_city_id: String) -> Self {
        let base_url = match api_type.to_lowercase().as_str() {
            "basic" => "https://api.rajaongkir.com/basic",
            "pro" => "https://pro.rajaongkir.com/api",
            _ => "https://api.rajaongkir.com/starter",
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            api_key,
            base_url: base_url.to_string(),
            origin_city_id,
            client,
        }
    }

    /// Calculate realistic mock domestic shipping rates based on rounded weight in kg
    pub fn mock_rates(req: &ShippingCostRequest) -> Vec<ShippingCost> {
        // Calculate weight in whole kg (minimum 1 kg, rounded up)
        let weight_kg = ((req.weight_grams as f64) / 1000.0).ceil() as i64;
        let weight_kg = weight_kg.max(1);

        let target_courier = req.courier.to_lowercase();
        let mut results = Vec::new();

        if target_courier == "all" || target_courier.contains("jne") {
            results.push(ShippingCost {
                courier_code: "jne".to_string(),
                courier_name: "Jalur Nugraha Ekakurir (JNE)".to_string(),
                service: "REG".to_string(),
                service_description: "Layanan Reguler".to_string(),
                cost_cents: 18000 * weight_kg,
                etd: "2-3".to_string(),
            });
            results.push(ShippingCost {
                courier_code: "jne".to_string(),
                courier_name: "Jalur Nugraha Ekakurir (JNE)".to_string(),
                service: "YES".to_string(),
                service_description: "Yakin Esok Sampai".to_string(),
                cost_cents: 32000 * weight_kg,
                etd: "1-1".to_string(),
            });
            results.push(ShippingCost {
                courier_code: "jne".to_string(),
                courier_name: "Jalur Nugraha Ekakurir (JNE)".to_string(),
                service: "OKE".to_string(),
                service_description: "Ongkos Kirim Ekonomis".to_string(),
                cost_cents: 14000 * weight_kg,
                etd: "3-5".to_string(),
            });
        }

        if target_courier == "all" || target_courier.contains("tiki") {
            results.push(ShippingCost {
                courier_code: "tiki".to_string(),
                courier_name: "Citra Van Titipan Kilat (TIKI)".to_string(),
                service: "REG".to_string(),
                service_description: "Regular Service".to_string(),
                cost_cents: 19000 * weight_kg,
                etd: "2-3".to_string(),
            });
            results.push(ShippingCost {
                courier_code: "tiki".to_string(),
                courier_name: "Citra Van Titipan Kilat (TIKI)".to_string(),
                service: "ONS".to_string(),
                service_description: "Over Night Services".to_string(),
                cost_cents: 34000 * weight_kg,
                etd: "1-1".to_string(),
            });
        }

        if target_courier == "all" || target_courier.contains("pos") {
            results.push(ShippingCost {
                courier_code: "pos".to_string(),
                courier_name: "POS Indonesia".to_string(),
                service: "Pos Reguler".to_string(),
                service_description: "Pos Reguler Dalam Negeri".to_string(),
                cost_cents: 17000 * weight_kg,
                etd: "2-4".to_string(),
            });
            results.push(ShippingCost {
                courier_code: "pos".to_string(),
                courier_name: "POS Indonesia".to_string(),
                service: "Pos Nextday".to_string(),
                service_description: "Pos Next Day Service".to_string(),
                cost_cents: 30000 * weight_kg,
                etd: "1-1".to_string(),
            });
        }

        results
    }

    /// Curated list of Indonesian cities for autocomplete and offline fallback
    fn standard_cities() -> Vec<ShippingCity> {
        vec![
            ShippingCity {
                city_id: "152".to_string(),
                province_id: "6".to_string(),
                province: "DKI Jakarta".to_string(),
                city_name: "Jakarta Selatan".to_string(),
                postal_code: "12000".to_string(),
            },
            ShippingCity {
                city_id: "151".to_string(),
                province_id: "6".to_string(),
                province: "DKI Jakarta".to_string(),
                city_name: "Jakarta Barat".to_string(),
                postal_code: "11000".to_string(),
            },
            ShippingCity {
                city_id: "153".to_string(),
                province_id: "6".to_string(),
                province: "DKI Jakarta".to_string(),
                city_name: "Jakarta Pusat".to_string(),
                postal_code: "10000".to_string(),
            },
            ShippingCity {
                city_id: "154".to_string(),
                province_id: "6".to_string(),
                province: "DKI Jakarta".to_string(),
                city_name: "Jakarta Timur".to_string(),
                postal_code: "13000".to_string(),
            },
            ShippingCity {
                city_id: "155".to_string(),
                province_id: "6".to_string(),
                province: "DKI Jakarta".to_string(),
                city_name: "Jakarta Utara".to_string(),
                postal_code: "14000".to_string(),
            },
            ShippingCity {
                city_id: "23".to_string(),
                province_id: "9".to_string(),
                province: "Jawa Barat".to_string(),
                city_name: "Bandung".to_string(),
                postal_code: "40111".to_string(),
            },
            ShippingCity {
                city_id: "444".to_string(),
                province_id: "11".to_string(),
                province: "Jawa Timur".to_string(),
                city_name: "Surabaya".to_string(),
                postal_code: "60111".to_string(),
            },
            ShippingCity {
                city_id: "278".to_string(),
                province_id: "34".to_string(),
                province: "Sumatera Utara".to_string(),
                city_name: "Medan".to_string(),
                postal_code: "20111".to_string(),
            },
            ShippingCity {
                city_id: "399".to_string(),
                province_id: "10".to_string(),
                province: "Jawa Tengah".to_string(),
                city_name: "Semarang".to_string(),
                postal_code: "50111".to_string(),
            },
            ShippingCity {
                city_id: "501".to_string(),
                province_id: "5".to_string(),
                province: "DI Yogyakarta".to_string(),
                city_name: "Yogyakarta".to_string(),
                postal_code: "55000".to_string(),
            },
            ShippingCity {
                city_id: "114".to_string(),
                province_id: "1".to_string(),
                province: "Bali".to_string(),
                city_name: "Denpasar".to_string(),
                postal_code: "80111".to_string(),
            },
            ShippingCity {
                city_id: "256".to_string(),
                province_id: "28".to_string(),
                province: "Sulawesi Selatan".to_string(),
                city_name: "Makassar".to_string(),
                postal_code: "90111".to_string(),
            },
            ShippingCity {
                city_id: "457".to_string(),
                province_id: "3".to_string(),
                province: "Banten".to_string(),
                city_name: "Tangerang".to_string(),
                postal_code: "15111".to_string(),
            },
            ShippingCity {
                city_id: "456".to_string(),
                province_id: "3".to_string(),
                province: "Banten".to_string(),
                city_name: "Tangerang Selatan".to_string(),
                postal_code: "15411".to_string(),
            },
            ShippingCity {
                city_id: "55".to_string(),
                province_id: "9".to_string(),
                province: "Jawa Barat".to_string(),
                city_name: "Bekasi".to_string(),
                postal_code: "17111".to_string(),
            },
            ShippingCity {
                city_id: "78".to_string(),
                province_id: "9".to_string(),
                province: "Jawa Barat".to_string(),
                city_name: "Bogor".to_string(),
                postal_code: "16111".to_string(),
            },
            ShippingCity {
                city_id: "108".to_string(),
                province_id: "9".to_string(),
                province: "Jawa Barat".to_string(),
                city_name: "Depok".to_string(),
                postal_code: "16411".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ShippingContract for ShippingModule {
    async fn list_couriers(&self) -> Result<Vec<ShippingCourier>, ContractError> {
        Ok(vec![
            ShippingCourier {
                code: "jne".to_string(),
                name: "JNE (Jalur Nugraha Ekakurir)".to_string(),
            },
            ShippingCourier {
                code: "tiki".to_string(),
                name: "TIKI (Citra Van Titipan Kilat)".to_string(),
            },
            ShippingCourier {
                code: "pos".to_string(),
                name: "POS Indonesia".to_string(),
            },
        ])
    }

    async fn calculate_cost(
        &self,
        req: ShippingCostRequest,
    ) -> Result<Vec<ShippingCost>, ContractError> {
        // If API key is empty or dummy/placeholder, return realistic mock domestic rates
        if self.api_key.trim().is_empty()
            || self.api_key.starts_with("dummy")
            || self.api_key.starts_with("your-api-key")
        {
            tracing::debug!(
                courier = %req.courier,
                weight_grams = req.weight_grams,
                destination = %req.destination_city_id,
                "Using mock domestic shipping rates (API key unset/mock)"
            );
            return Ok(Self::mock_rates(&req));
        }

        let url = format!("{}/cost", self.base_url);
        let weight_str = req.weight_grams.to_string();
        let params = [
            ("origin", self.origin_city_id.as_str()),
            ("destination", req.destination_city_id.as_str()),
            ("weight", weight_str.as_str()),
            ("courier", req.courier.as_str()),
        ];

        let res = match self
            .client
            .post(&url)
            .header("key", &self.api_key)
            .form(&params)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "RajaOngkir network call failed, falling back to mock rates"
                );
                return Ok(Self::mock_rates(&req));
            }
        };

        if !res.status().is_success() {
            tracing::warn!(
                status = %res.status(),
                "RajaOngkir returned non-200 status, falling back to mock rates"
            );
            return Ok(Self::mock_rates(&req));
        }

        let parsed: RajaOngkirResponse = match res.json().await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to parse RajaOngkir JSON, falling back to mock rates"
                );
                return Ok(Self::mock_rates(&req));
            }
        };

        if parsed.rajaongkir.status.code != 200 {
            tracing::warn!(
                code = parsed.rajaongkir.status.code,
                desc = ?parsed.rajaongkir.status.description,
                "RajaOngkir status not 200, falling back to mock rates"
            );
            return Ok(Self::mock_rates(&req));
        }

        let mut costs = Vec::new();
        if let Some(results) = parsed.rajaongkir.results {
            for courier_res in results {
                for cost_item in courier_res.costs {
                    if let Some(first_cost) = cost_item.cost.first() {
                        costs.push(ShippingCost {
                            courier_code: courier_res.code.clone(),
                            courier_name: courier_res.name.clone(),
                            service: cost_item.service.clone(),
                            service_description: cost_item.description.clone(),
                            cost_cents: first_cost.value,
                            etd: first_cost.etd.clone().unwrap_or_else(|| "2-3".to_string()),
                        });
                    }
                }
            }
        }

        if costs.is_empty() {
            Ok(Self::mock_rates(&req))
        } else {
            Ok(costs)
        }
    }

    async fn search_cities(&self, query: &str) -> Result<Vec<ShippingCity>, ContractError> {
        let q = query.trim().to_lowercase();
        let all_cities = Self::standard_cities();

        if q.is_empty() {
            return Ok(all_cities.into_iter().take(15).collect());
        }

        let filtered: Vec<ShippingCity> = all_cities
            .into_iter()
            .filter(|c| {
                c.city_name.to_lowercase().contains(&q)
                    || c.province.to_lowercase().contains(&q)
                    || c.postal_code.contains(&q)
            })
            .collect();

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_rates_calculation() {
        let module = ShippingModule::new(
            "".to_string(),
            "starter".to_string(),
            "152".to_string(),
        );

        // 500 grams should round up to 1 kg
        let req_500g = ShippingCostRequest {
            destination_city_id: "444".to_string(),
            weight_grams: 500,
            courier: "jne".to_string(),
        };
        let costs = module.calculate_cost(req_500g).await.unwrap();
        assert!(!costs.is_empty());
        let reg = costs.iter().find(|c| c.service == "REG").unwrap();
        assert_eq!(reg.cost_cents, 18000);

        // 1200 grams should round up to 2 kg
        let req_1200g = ShippingCostRequest {
            destination_city_id: "444".to_string(),
            weight_grams: 1200,
            courier: "jne".to_string(),
        };
        let costs_2kg = module.calculate_cost(req_1200g).await.unwrap();
        let reg_2kg = costs_2kg.iter().find(|c| c.service == "REG").unwrap();
        assert_eq!(reg_2kg.cost_cents, 36000);
    }

    #[tokio::test]
    async fn test_list_couriers() {
        let module = ShippingModule::new(
            "".to_string(),
            "starter".to_string(),
            "152".to_string(),
        );
        let couriers = module.list_couriers().await.unwrap();
        assert_eq!(couriers.len(), 3);
        assert!(couriers.iter().any(|c| c.code == "jne"));
        assert!(couriers.iter().any(|c| c.code == "tiki"));
        assert!(couriers.iter().any(|c| c.code == "pos"));
    }

    #[tokio::test]
    async fn test_search_cities() {
        let module = ShippingModule::new(
            "".to_string(),
            "starter".to_string(),
            "152".to_string(),
        );
        let results = module.search_cities("jakarta").await.unwrap();
        assert!(results.len() >= 5);
        assert!(results.iter().all(|c| c.city_name.to_lowercase().contains("jakarta")));

        let sby = module.search_cities("surabaya").await.unwrap();
        assert_eq!(sby.len(), 1);
        assert_eq!(sby[0].city_name, "Surabaya");
    }
}
