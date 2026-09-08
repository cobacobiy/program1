use chrono::Utc;
use program1_contracts::{
    BuyerAccountDto, BuyerAddressDto, ChatMessageDto, ChatRoomDto, ChatSenderType, JwtClaims,
    SendMessageRequest, ShippingAddressSnapshot,
};
use uuid::Uuid;

#[test]
fn test_buyer_contract_types_instantiation() {
    let buyer_id = Uuid::new_v4();
    let now = Utc::now();
    let buyer = BuyerAccountDto {
        id: buyer_id,
        google_sub: Some("google-123456".to_string()),
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
        courier: None,
        shipping_cost_cents: None,
    };
    assert!(valid_req.validate().is_ok());

    let empty_items_req = BuyerCheckoutRequest {
        address_id,
        items: vec![],
        courier: None,
        shipping_cost_cents: None,
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
fn test_register_buyer_request_validation() {
    use program1_contracts::RegisterBuyerRequest;
    use validator::Validate;

    let valid = RegisterBuyerRequest {
        full_name: "Budi Santoso".to_string(),
        email: "budi@example.com".to_string(),
        password: "Password123!".to_string(),
    };
    assert!(valid.validate().is_ok());

    let short_name = RegisterBuyerRequest {
        full_name: "B".to_string(),
        email: "budi@example.com".to_string(),
        password: "Password123!".to_string(),
    };
    assert!(short_name.validate().is_err());

    let invalid_email = RegisterBuyerRequest {
        full_name: "Budi Santoso".to_string(),
        email: "not-an-email".to_string(),
        password: "Password123!".to_string(),
    };
    assert!(invalid_email.validate().is_err());

    let short_password = RegisterBuyerRequest {
        full_name: "Budi Santoso".to_string(),
        email: "budi@example.com".to_string(),
        password: "short".to_string(),
    };
    assert!(short_password.validate().is_err());
}

#[test]
fn test_buyer_login_request_validation() {
    use program1_contracts::BuyerLoginRequest;
    use validator::Validate;

    let valid = BuyerLoginRequest {
        email: "budi@example.com".to_string(),
        password: "Password123!".to_string(),
    };
    assert!(valid.validate().is_ok());

    let invalid_email = BuyerLoginRequest {
        email: "invalid-email".to_string(),
        password: "Password123!".to_string(),
    };
    assert!(invalid_email.validate().is_err());

    let empty_password = BuyerLoginRequest {
        email: "budi@example.com".to_string(),
        password: "".to_string(),
    };
    assert!(empty_password.validate().is_err());
}

#[test]
fn test_static_frontend_store_js_contract_parity() {
    let static_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("web")
        .join("static");

    let auth_content = std::fs::read_to_string(static_dir.join("store").join("store-auth.js"))
        .or_else(|_| std::fs::read_to_string(static_dir.join("store.js")))
        .expect("Failed to read store-auth.js or store.js");

    let checkout_content =
        std::fs::read_to_string(static_dir.join("store").join("store-checkout.js"))
            .or_else(|_| std::fs::read_to_string(static_dir.join("store.js")))
            .expect("Failed to read store-checkout.js or store.js");

    // 1. OTP verify must send field 'code' not 'otp_code'
    assert!(
        auth_content.contains("code: code"),
        "store-auth.js OTP verify must send 'code' field to match OtpVerifyRequest"
    );
    assert!(
        !auth_content.contains("otp_code:"),
        "store-auth.js must NOT send 'otp_code' field"
    );

    // 2. Address create/update must send 'set_as_default' not 'is_default' in mutation payload
    assert!(
        checkout_content.contains("set_as_default:"),
        "store-checkout.js address save payload must send 'set_as_default' field"
    );

    // 3. OTP verify response handler must handle BuyerAccountDto directly
    assert!(
        auth_content.contains("activeBuyer = result.id ? result : (result.buyer || result)")
            || auth_content.contains(
                "activeBuyer = (result && result.id) ? result : (result.buyer || result)"
            )
            || auth_content.contains("activeBuyer = result;"),
        "store-auth.js OTP verify must handle BuyerAccountDto directly rather than solely result.buyer"
    );
}

#[test]
fn test_buyer_orders_modal_review_routes_parity() {
    let frontend_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("frontend")
        .join("src")
        .join("lib");

    let modal_content = std::fs::read_to_string(frontend_dir.join("BuyerOrdersModal.svelte"))
        .expect("Failed to read BuyerOrdersModal.svelte");

    // All review fetch and submit calls in BuyerOrdersModal must target '/api/v1/buyer/reviews'
    assert!(
        modal_content.contains("'/api/v1/buyer/reviews'"),
        "BuyerOrdersModal.svelte must target canonical endpoint '/api/v1/buyer/reviews'"
    );

    // Guard against bare un-prefixed '/buyer/reviews' path
    assert!(
        !modal_content.contains("'/buyer/reviews'"),
        "BuyerOrdersModal.svelte must NOT use bare '/buyer/reviews' route without '/api/v1'"
    );
}

#[test]
fn test_chat_contract_types_and_validation() {
    use validator::Validate;

    let room_id = Uuid::new_v4();
    let sender_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();
    let now = Utc::now();

    let msg = ChatMessageDto {
        id: msg_id,
        room_id,
        sender_type: ChatSenderType::Buyer,
        sender_id,
        sender_name: "Budi Santoso".to_string(),
        content: "Halo admin, apakah stok produk ini ready?".to_string(),
        is_read: false,
        created_at: now,
    };

    assert_eq!(msg.sender_type, ChatSenderType::Buyer);
    assert_eq!(msg.sender_name, "Budi Santoso");
    assert!(!msg.is_read);

    let room = ChatRoomDto {
        id: room_id,
        buyer_id: sender_id,
        buyer_name: "Budi Santoso".to_string(),
        last_message: Some(msg.content.clone()),
        unread_count: 1,
        updated_at: now,
        created_at: now,
    };

    assert_eq!(room.unread_count, 1);
    assert_eq!(room.buyer_name, "Budi Santoso");

    // Validation for SendMessageRequest
    let valid_req = SendMessageRequest {
        content: "Tanya produk".to_string(),
    };
    assert!(valid_req.validate().is_ok());

    let empty_req = SendMessageRequest {
        content: "".to_string(),
    };
    assert!(empty_req.validate().is_err());

    let too_long_req = SendMessageRequest {
        content: "a".repeat(2001),
    };
    assert!(too_long_req.validate().is_err());
}

#[test]
fn test_payment_contract_types_and_validation() {
    use program1_contracts::{CreatePaymentRequest, PaymentConfigDto, PaymentTransactionDto};
    use validator::Validate;

    let payment_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let now = Utc::now();

    let tx = PaymentTransactionDto {
        id: payment_id,
        order_id,
        payment_method: "qris".to_string(),
        amount: 150000.0,
        currency: "IDR".to_string(),
        status: "pending".to_string(),
        provider_ref: Some("mid-trans-123".to_string()),
        snap_token: Some("snap-token-abc".to_string()),
        snap_redirect_url: Some("https://app.sandbox.midtrans.com/snap/v2/vtweb/abc".to_string()),
        paid_at: None,
        created_at: now,
    };

    assert_eq!(tx.id, payment_id);
    assert_eq!(tx.amount, 150000.0);
    assert_eq!(tx.status, "pending");

    let req = CreatePaymentRequest { order_id };
    assert!(req.validate().is_ok());

    let config = PaymentConfigDto {
        client_key: "SB-Mid-client-xxx".to_string(),
        is_production: false,
        snap_url: "https://app.sandbox.midtrans.com/snap/snap.js".to_string(),
    };
    assert_eq!(config.client_key, "SB-Mid-client-xxx");
    assert!(!config.is_production);
}

#[test]
fn test_order_status_transitions_and_validation() {
    use program1_contracts::{CancelOrderRequest, OrderStatus, UpdateOrderStatusRequest};
    use validator::Validate;

    // Test as_str and from_str
    assert_eq!(OrderStatus::Pending.as_str(), "pending");
    assert_eq!(OrderStatus::Paid.as_str(), "paid");
    assert_eq!(OrderStatus::Processing.as_str(), "processing");
    assert_eq!(OrderStatus::Shipped.as_str(), "shipped");
    assert_eq!(OrderStatus::Delivered.as_str(), "delivered");
    assert_eq!(OrderStatus::Completed.as_str(), "completed");
    assert_eq!(OrderStatus::Cancelled.as_str(), "cancelled");
    assert_eq!(OrderStatus::ReturnRequested.as_str(), "return_requested");
    assert_eq!(OrderStatus::Returned.as_str(), "returned");

    assert_eq!(OrderStatus::from_str("pending"), Some(OrderStatus::Pending));
    assert_eq!(OrderStatus::from_str("PAID"), Some(OrderStatus::Paid));
    assert_eq!(OrderStatus::from_str("shipped"), Some(OrderStatus::Shipped));
    assert_eq!(OrderStatus::from_str("invalid"), None);

    // Test valid transitions: pending -> paid -> processing -> shipped -> delivered -> completed
    assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Paid));
    assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Cancelled));
    assert!(OrderStatus::Paid.can_transition_to(&OrderStatus::Processing));
    assert!(OrderStatus::Paid.can_transition_to(&OrderStatus::Cancelled));
    assert!(OrderStatus::Processing.can_transition_to(&OrderStatus::Shipped));
    assert!(OrderStatus::Shipped.can_transition_to(&OrderStatus::Delivered));
    assert!(OrderStatus::Delivered.can_transition_to(&OrderStatus::Completed));
    assert!(OrderStatus::Delivered.can_transition_to(&OrderStatus::ReturnRequested));
    assert!(OrderStatus::ReturnRequested.can_transition_to(&OrderStatus::Returned));
    assert!(OrderStatus::ReturnRequested.can_transition_to(&OrderStatus::Completed));

    // Test invalid transitions
    assert!(!OrderStatus::Pending.can_transition_to(&OrderStatus::Shipped));
    assert!(!OrderStatus::Pending.can_transition_to(&OrderStatus::Completed));
    assert!(!OrderStatus::Cancelled.can_transition_to(&OrderStatus::Processing));
    assert!(!OrderStatus::Completed.can_transition_to(&OrderStatus::Shipped));

    // Test request validation
    let valid_update = UpdateOrderStatusRequest {
        new_status: OrderStatus::Shipped,
        tracking_number: Some("JNT123456789".to_string()),
        reason: None,
    };
    assert!(valid_update.validate().is_ok());

    let long_tracking = UpdateOrderStatusRequest {
        new_status: OrderStatus::Shipped,
        tracking_number: Some("a".repeat(101)),
        reason: None,
    };
    assert!(long_tracking.validate().is_err());

    let cancel_req = CancelOrderRequest {
        reason: Some("Salah ukuran".to_string()),
    };
    assert!(cancel_req.validate().is_ok());
}

#[test]
fn test_public_review_dto_privacy_minimization() {
    use program1_contracts::PublicReviewDto;

    let review_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let now = Utc::now();

    let public_dto = PublicReviewDto {
        id: review_id,
        product_id,
        buyer_name: "John Doe".to_string(),
        rating: 5,
        review_text: Some("Sangat bagus dan recommended!".to_string()),
        created_at: now,
        updated_at: now,
    };

    let serialized = serde_json::to_value(&public_dto).expect("Serialize PublicReviewDto");
    // Verify privacy-safe fields are present
    assert_eq!(serialized["id"], review_id.to_string());
    assert_eq!(serialized["product_id"], product_id.to_string());
    assert_eq!(serialized["buyer_name"], "John Doe");
    assert_eq!(serialized["rating"], 5);
    assert_eq!(serialized["review_text"], "Sangat bagus dan recommended!");
    assert!(serialized.get("created_at").is_some());
    assert!(serialized.get("updated_at").is_some());

    // Verify sensitive and internal fields are completely absent
    assert!(serialized.get("buyer_id").is_none());
    assert!(serialized.get("order_id").is_none());
    assert!(serialized.get("is_visible").is_none());
    assert!(serialized.get("email").is_none());
    assert!(serialized.get("phone").is_none());
    assert!(serialized.get("phone_number").is_none());
}

#[test]
fn test_shipping_contract_types() {
    use program1_contracts::{ShippingCity, ShippingCost, ShippingCostRequest, ShippingCourier, UpdateOrderTrackingRequest};
    use validator::Validate;

    // Valid ShippingCostRequest
    let req = ShippingCostRequest {
        destination_city_id: "152".to_string(),
        weight_grams: 1000,
        courier: "jne".to_string(),
    };
    assert!(req.validate().is_ok());

    // Invalid weight
    let invalid_weight = ShippingCostRequest {
        destination_city_id: "152".to_string(),
        weight_grams: 0,
        courier: "jne".to_string(),
    };
    assert!(invalid_weight.validate().is_err());

    // Empty city
    let empty_city = ShippingCostRequest {
        destination_city_id: "".to_string(),
        weight_grams: 500,
        courier: "jne".to_string(),
    };
    assert!(empty_city.validate().is_err());

    // Update tracking request
    let valid_tracking = UpdateOrderTrackingRequest {
        tracking_number: "JNE12345678".to_string(),
    };
    assert!(valid_tracking.validate().is_ok());

    let empty_tracking = UpdateOrderTrackingRequest {
        tracking_number: "".to_string(),
    };
    assert!(empty_tracking.validate().is_err());

    let courier = ShippingCourier {
        code: "jne".to_string(),
        name: "Jalur Nugraha Ekakurir".to_string(),
    };
    assert_eq!(courier.code, "jne");

    let cost = ShippingCost {
        courier_code: "jne".to_string(),
        courier_name: "Jalur Nugraha Ekakurir".to_string(),
        service: "REG".to_string(),
        service_description: "Reguler".to_string(),
        cost_cents: 18000,
        etd: "2-3".to_string(),
    };
    assert_eq!(cost.cost_cents, 18000);

    let city = ShippingCity {
        city_id: "152".to_string(),
        province_id: "6".to_string(),
        province: "DKI Jakarta".to_string(),
        city_name: "Jakarta Selatan".to_string(),
        postal_code: "12000".to_string(),
    };
    assert_eq!(city.city_id, "152");
}

