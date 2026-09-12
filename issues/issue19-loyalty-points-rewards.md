# Issue 19: Customer Loyalty Points & Membership Tier System (Phase 4 — #2)

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Selesai Issue #36 (Order Status Workflow) & Issue #41 (Buyer Profile)  

---

## 📋 Deskripsi

Untuk meningkatkan retensi pembeli (*repeat purchase*), e-commerce membutuhkan sistem loyalitas pelanggan.  
Issue ini mengimplementasikan:
1. **Poin Belanja (Cashback Points)**: Pembeli memperoleh poin (contoh: 1% dari total pesanan, misal Rp 100.000 = 1.000 poin) ketika pesanan berhasil diselesaikan (`delivered` / `completed`).
2. **Tingkatan Keanggotaan (Membership Tiers)**: `Classic`, `Silver`, `Gold`, `Platinum` berdasarkan total akumulasi belanja pembeli.
3. **Penukaran Poin di Checkout**: Pembeli dapat menukarkan saldo poin mereka untuk memotong total pembayaran belanja (1 poin = Rp 1).
4. **Riwayat Mutasi Poin**: Catatan transparansi perolehan dan penggunaan poin di profil pembeli.

---

## 🎯 Acceptance Criteria

- [ ] Setiap pesanan dengan status `delivered` / `confirmed` otomatis mengkreditkan poin loyalitas ke akun pembeli.
- [ ] Profil pembeli di modal/dashboard menampilkan saldo poin aktif, riwayat transaksi poin, dan badge tingkatan tier (Classic/Silver/Gold/Platinum).
- [ ] Di modal Checkout, pembeli dapat mencentang opsi "Gunakan Poin Saya" (maksimal sesuai saldo dan tidak melebihi subtotal pesanan).
- [ ] Pemotongan poin tercatat di rincian order (`points_redeemed_cents`).
- [ ] Jika pesanan dibatalkan atau dikembalikan (*refund*), poin yang diperoleh dari pesanan tersebut dibatalkan/ditarik kembali (*point clawback*), dan poin yang dipakai belanja dikembalikan.
- [ ] Minimal 3 unit/integration test mencakup: perhitungan perolehan poin, penggunaan poin saat checkout, dan rollback poin saat pesanan dibatalkan.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Migrasi Database
**File:** `migrations/sqlite/022_create_loyalty_points.sql`

```sql
ALTER TABLE buyers ADD COLUMN points_balance INTEGER NOT NULL DEFAULT 0;
ALTER TABLE buyers ADD COLUMN membership_tier TEXT NOT NULL DEFAULT 'Classic';

CREATE TABLE IF NOT EXISTS loyalty_point_ledgers (
    id TEXT PRIMARY KEY NOT NULL,
    buyer_id TEXT NOT NULL REFERENCES buyers(id) ON DELETE CASCADE,
    order_id TEXT REFERENCES orders(id) ON DELETE SET NULL,
    points_delta INTEGER NOT NULL,          -- Positif (earned) atau negatif (redeemed)
    balance_after INTEGER NOT NULL,
    description TEXT NOT NULL,              -- contoh: "Cashback pesanan #ORD-12345"
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_loyalty_buyer ON loyalty_point_ledgers(buyer_id, created_at DESC);
```

### Langkah 2: Kontrak Interface & DTO
**File:** `crates/contracts/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoyaltySummaryDto {
    pub points_balance: i64,
    pub membership_tier: String,
    pub total_earned_all_time: i64,
    pub ledgers: Vec<LoyaltyLedgerEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoyaltyLedgerEntryDto {
    pub id: String,
    pub points_delta: i64,
    pub balance_after: i64,
    pub description: String,
    pub created_at: String,
}
```

### Langkah 3: Integrasi Order Hook
**File:** `crates/modules/order/src/lib.rs`
- Saat order transition ke `Delivered` atau `Confirmed`:
  Hitung poin: `let points = (order.total_amount_cents / 100).max(0);`
  Tambahkan transaksi kredit ke ledger poin pembeli.
- Saat checkout:
  Jika `request.use_points == true`, kurangi total tagihan sebesar poin yang valid dan potong saldo poin pembeli secara atomik.

### Langkah 4: Tampilan Frontend Svelte 5
**File:** `frontend/src/lib/CheckoutModal.svelte` & `frontend/src/App.svelte`
- Tampilkan saldo poin pembeli di checkout dengan toggle checkbox: "⭐ Tukarkan 5.000 Poin (Hemat Rp 5.000)".
- Tampilkan badge keanggotaan di profil akun pembeli (misal: "👑 Member Gold").

### Langkah 5: Unit & Integration Tests
**File:** `crates/web/tests/loyalty_test.rs`
- Uji perolehan poin saat order selesai.
- Uji checkout dengan pemotongan poin.
- Uji isolasi antar akun pembeli (tidak bisa memakai poin milik orang lain).
