use chrono::Utc;
use program1_contracts::{BuyerAccountDto, BuyerAddressDto, JwtClaims, ShippingAddressSnapshot};
use uuid::Uuid;

#[test]
fn test_buyer_contract_types_instantiation() {
    let buyer_id = Uuid::new_v4();
    let now = Utc::now();
    let buyer = BuyerAccountDto {
        id: buyer_id,
        google_sub: "google-123456".to_string(),
        email: "buyer@example.com".to_string(),
        full_name: "Buyer Jane".to_string(),
        avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        phone_number: Some("+628123456789".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: now,
        updated_at: now,
    };
    assert_eq!(buyer.email, "buyer@example.com");
    assert!(buyer.phone_verified);

    let address = BuyerAddressDto {
        id: Uuid::new_v4(),
        buyer_id,
        recipient_name: "Buyer Jane".to_string(),
        phone_number: "+628123456789".to_string(),
        street_address: "Jl. Sudirman 100".to_string(),
        subdistrict: "Kebayoran Baru".to_string(),
        city: "Jakarta Selatan".to_string(),
        province: "DKI Jakarta".to_string(),
        postal_code: "12190".to_string(),
        is_default: true,
        created_at: now,
    };
    assert!(address.is_default);

    let snapshot = ShippingAddressSnapshot {
        recipient_name: address.recipient_name.clone(),
        phone_number: address.phone_number.clone(),
        street_address: address.street_address.clone(),
        subdistrict: address.subdistrict.clone(),
        city: address.city.clone(),
        province: address.province.clone(),
        postal_code: address.postal_code.clone(),
    };
    assert_eq!(snapshot.postal_code, "12190");
}

#[test]
fn test_jwt_claims_buyer_vs_seller() {
    let seller_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "admin".to_string(),
        role: "Super Admin".to_string(),
        accessible_menus: vec!["dashboard".to_string()],
        exp: 9999999999,
        iat: 1000000000,
        user_type: "seller_staff".to_string(),
    };
    assert!(seller_claims.is_seller_staff());
    assert!(!seller_claims.is_buyer());

    let buyer_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "buyer@example.com".to_string(),
        role: "Buyer".to_string(),
        accessible_menus: vec![],
        exp: 9999999999,
        iat: 1000000000,
        user_type: "buyer".to_string(),
    };
    assert!(buyer_claims.is_buyer());
    assert!(!buyer_claims.is_seller_staff());
}

#[test]
fn test_buyer_auth_response_serializes_access_token_and_deserializes_from_token_alias() {
    use program1_contracts::BuyerAuthResponse;

    let json_with_access_token = serde_json::json!({
        "access_token": "jwt-token-123",
        "token_type": "Bearer",
        "expires_in": 86400,
        "buyer": {
            "id": Uuid::new_v4(),
            "google_sub": "sub-123",
            "email": "test@buyer.com",
            "full_name": "Test Buyer",
            "avatar_url": null,
            "phone_number": null,
            "phone_verified": false,
            "created_at": Utc::now(),
            "updated_at": Utc::now()
        },
        "requires_phone_verification": true
    });

    let parsed1: BuyerAuthResponse = serde_json::from_value(json_with_access_token).unwrap();
    assert_eq!(parsed1.access_token, "jwt-token-123");

    // Also verify deserialization succeeds when payload provides "token" instead of "access_token"
    let json_with_token_alias = serde_json::json!({
        "token": "jwt-token-456",
        "token_type": "Bearer",
        "expires_in": 86400,
        "buyer": {
            "id": Uuid::new_v4(),
            "google_sub": "sub-456",
            "email": "test456@buyer.com",
            "full_name": "Test Buyer 2",
            "avatar_url": null,
            "phone_number": null,
            "phone_verified": false,
            "created_at": Utc::now(),
            "updated_at": Utc::now()
        },
        "requires_phone_verification": true
    });

    let parsed2: BuyerAuthResponse = serde_json::from_value(json_with_token_alias).unwrap();
    assert_eq!(parsed2.access_token, "jwt-token-456");

    // Verify serialization produces "access_token"
    let serialized = serde_json::to_string(&parsed2).unwrap();
    assert!(serialized.contains("\"access_token\":"));
}

#[test]
fn test_buyer_checkout_request_structure() {
    use program1_contracts::{BuyerCheckoutRequest, StorefrontOrderItemRequest};
    use validator::Validate;

    let address_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let valid_req = BuyerCheckoutRequest {
        address_id,
        items: vec![StorefrontOrderItemRequest {
            product_id,
            quantity: 2,
        }],
    };
    assert!(valid_req.validate().is_ok());

    let empty_items_req = BuyerCheckoutRequest {
        address_id,
        items: vec![],
    };
    assert!(empty_items_req.validate().is_err());
}

#[test]
fn test_otp_verify_contract_code_field_and_alias() {
    use program1_contracts::OtpVerifyRequest;
    use validator::Validate;

    // Canonical frontend payload: "code"
    let json_canonical = serde_json::json!({
        "phone_number": "+628123456789",
        "code": "123456"
    });
    let parsed1: Result<OtpVerifyRequest, _> = serde_json::from_value(json_canonical);
    assert!(parsed1.is_ok());
    let req1 = parsed1.unwrap();
    assert_eq!(req1.code, "123456");
    assert!(req1.validate().is_ok());

    // Backwards-compatible payload: "otp_code" alias
    let json_alias = serde_json::json!({
        "phone_number": "+628123456789",
        "otp_code": "654321"
    });
    let parsed2: Result<OtpVerifyRequest, _> = serde_json::from_value(json_alias);
    assert!(parsed2.is_ok());
    let req2 = parsed2.unwrap();
    assert_eq!(req2.code, "654321");
    assert!(req2.validate().is_ok());
}

#[test]
fn test_address_contract_set_as_default_field_and_alias() {
    use program1_contracts::{CreateBuyerAddressRequest, UpdateBuyerAddressRequest};
    use validator::Validate;

    // Canonical frontend payload: "set_as_default"
    let json_create_canonical = serde_json::json!({
        "recipient_name": "Budi Santoso",
        "phone_number": "+628123456789",
        "street_address": "Jl. MH Thamrin No. 1",
        "subdistrict": "Menteng",
        "city": "Jakarta Pusat",
        "province": "DKI Jakarta",
        "postal_code": "10310",
        "set_as_default": true
    });
    let parsed_create: CreateBuyerAddressRequest =
        serde_json::from_value(json_create_canonical).unwrap();
    assert!(parsed_create.set_as_default);
    assert!(parsed_create.validate().is_ok());

    // Dual-compatibility alias: "is_default"
    let json_create_alias = serde_json::json!({
        "recipient_name": "Budi Santoso",
        "phone_number": "+628123456789",
        "street_address": "Jl. MH Thamrin No. 1",
        "subdistrict": "Menteng",
        "city": "Jakarta Pusat",
        "province": "DKI Jakarta",
        "postal_code": "10310",
        "is_default": true
    });
    let parsed_alias: CreateBuyerAddressRequest =
        serde_json::from_value(json_create_alias).unwrap();
    assert!(parsed_alias.set_as_default);
    assert!(parsed_alias.validate().is_ok());

    // Update address request canonical
    let json_update_canonical = serde_json::json!({
        "recipient_name": "Budi Santoso",
        "phone_number": "+628123456789",
        "street_address": "Jl. MH Thamrin No. 1",
        "subdistrict": "Menteng",
        "city": "Jakarta Pusat",
        "province": "DKI Jakarta",
        "postal_code": "10310",
        "set_as_default": false
    });
    let parsed_update: UpdateBuyerAddressRequest =
        serde_json::from_value(json_update_canonical).unwrap();
    assert!(!parsed_update.set_as_default);
    assert!(parsed_update.validate().is_ok());
}

#[test]
fn test_buyer_account_dto_is_active_default() {
    let raw = serde_json::json!({
        "id": Uuid::new_v4(),
        "google_sub": "sub-test",
        "email": "active@buyer.com",
        "full_name": "Active Buyer",
        "avatar_url": null,
        "phone_number": null,
        "phone_verified": false,
        "created_at": Utc::now(),
        "updated_at": Utc::now()
    });
    let dto: BuyerAccountDto = serde_json::from_value(raw).unwrap();
    assert!(
        dto.is_active,
        "BuyerAccountDto without explicit is_active should default to true"
    );
}

#[test]
fn test_static_frontend_store_js_contract_parity() {
    let store_js_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("web")
        .join("static")
        .join("store.js");

    assert!(
        store_js_path.exists(),
        "store.js must exist at {:?}",
        store_js_path
    );
    let content = std::fs::read_to_string(&store_js_path).expect("Failed to read store.js");

    // 1. OTP verify must send field 'code' not 'otp_code'
    assert!(
        content.contains("code: code"),
        "store.js OTP verify must send 'code' field to match OtpVerifyRequest"
    );
    assert!(
        !content.contains("otp_code:"),
        "store.js must NOT send 'otp_code' field"
    );

    // 2. Address create/update must send 'set_as_default' not 'is_default' in mutation payload
    assert!(
        content.contains("set_as_default:"),
        "store.js address save payload must send 'set_as_default' field"
    );

    // 3. OTP verify response handler must handle BuyerAccountDto directly
    assert!(
        content.contains("activeBuyer = result.id ? result : (result.buyer || result)")
            || content.contains(
                "activeBuyer = (result && result.id) ? result : (result.buyer || result)"
            )
            || content.contains("activeBuyer = result;"),
        "store.js OTP verify must handle BuyerAccountDto directly rather than solely result.buyer"
    );
}
