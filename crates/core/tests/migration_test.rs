use program1_core::database::init_database;

#[tokio::test]
async fn test_migration_008_creates_buyer_and_breakglass_tables() {
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Failed to initialize database");

    // Check buyer_accounts table
    let buyer_table_check = sqlx::query("SELECT COUNT(*) FROM buyer_accounts")
        .fetch_one(&pool)
        .await;
    assert!(
        buyer_table_check.is_ok(),
        "buyer_accounts table should exist"
    );

    // Check buyer_addresses table
    let address_table_check = sqlx::query("SELECT COUNT(*) FROM buyer_addresses")
        .fetch_one(&pool)
        .await;
    assert!(
        address_table_check.is_ok(),
        "buyer_addresses table should exist"
    );

    // Check buyer_otp_verifications table
    let otp_table_check = sqlx::query("SELECT COUNT(*) FROM buyer_otp_verifications")
        .fetch_one(&pool)
        .await;
    assert!(
        otp_table_check.is_ok(),
        "buyer_otp_verifications table should exist"
    );

    // Check break_glass_sessions table
    let break_glass_check = sqlx::query("SELECT COUNT(*) FROM break_glass_sessions")
        .fetch_one(&pool)
        .await;
    assert!(
        break_glass_check.is_ok(),
        "break_glass_sessions table should exist"
    );

    // Check orders table columns
    let orders_col_check =
        sqlx::query("SELECT shipping_snapshot_json, buyer_id FROM orders LIMIT 1")
            .fetch_optional(&pool)
            .await;
    assert!(
        orders_col_check.is_ok(),
        "orders should have shipping_snapshot_json and buyer_id columns"
    );
}

#[tokio::test]
async fn test_migration_008_partial_unique_index_enforces_single_default_address() {
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Failed to initialize database");

    // Insert dummy buyer
    sqlx::query("INSERT INTO buyer_accounts (id, google_sub, email, full_name) VALUES ('b1', 'gsub1', 'b1@test.com', 'Buyer 1')")
        .execute(&pool)
        .await
        .unwrap();

    // Insert first default address
    let res1 = sqlx::query("INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, is_default) VALUES ('a1', 'b1', 'B1', '081234', 'Street 1', 1)")
        .execute(&pool)
        .await;
    assert!(res1.is_ok());

    // Insert second default address for SAME buyer -> must fail due to unique index!
    let res2 = sqlx::query("INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, is_default) VALUES ('a2', 'b1', 'B1', '081234', 'Street 2', 1)")
        .execute(&pool)
        .await;
    assert!(
        res2.is_err(),
        "Partial unique index must prevent two default addresses for same buyer"
    );

    // Insert second non-default address for SAME buyer -> must succeed!
    let res3 = sqlx::query("INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, is_default) VALUES ('a3', 'b1', 'B1', '081234', 'Street 3', 0)")
        .execute(&pool)
        .await;
    assert!(res3.is_ok());
}

#[tokio::test]
async fn test_migration_008_supports_email_password_buyer_with_null_google_sub() {
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Failed to initialize database");

    // Insert buyer without google_sub (NULL) and with password_hash
    let res = sqlx::query(
        "INSERT INTO buyer_accounts (id, google_sub, email, password_hash, full_name)
         VALUES ('b_email1', NULL, 'buyer_email@test.com', 'argon2_hash_dummy', 'Buyer Email 1')",
    )
    .execute(&pool)
    .await;
    assert!(res.is_ok(), "Inserting buyer with NULL google_sub and password_hash must succeed");

    // Multiple buyers with NULL google_sub should succeed (NULL is distinct in UNIQUE constraint)
    let res2 = sqlx::query(
        "INSERT INTO buyer_accounts (id, google_sub, email, password_hash, full_name)
         VALUES ('b_email2', NULL, 'buyer_email2@test.com', 'argon2_hash_dummy_2', 'Buyer Email 2')",
    )
    .execute(&pool)
    .await;
    assert!(res2.is_ok(), "Multiple buyers with NULL google_sub must succeed without violating UNIQUE index");
}
