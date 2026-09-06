# Issue 6: Admin Dashboard — Laporan Penjualan & Export CSV/PDF

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 2-3 hari  
> **Kesulitan:** ⭐⭐ Mudah  
> **Prerequisite:** Analytics module sudah ada (sudah DONE)

---

## 📋 Deskripsi

Admin saat ini hanya bisa lihat analytics summary sederhana.  
Issue ini menambahkan fitur **laporan penjualan detail** dengan:
- Filter berdasarkan tanggal (dari-sampai)
- Tabel detail per order
- Export ke CSV
- Cetak laporan (print-friendly PDF)

---

## 🎯 Acceptance Criteria

- [ ] Admin bisa filter laporan by date range
- [ ] Tampilkan tabel: tanggal, order ID, buyer, produk, qty, total, status
- [ ] Tombol "Export CSV" untuk download data
- [ ] Tombol "Cetak" dengan layout print-friendly
- [ ] Summary cards: total pendapatan, jumlah order, average order value
- [ ] Unit test minimal 2 test case

---

## 📐 Langkah-Langkah

### Langkah 1: Tambah Endpoint Report di Backend

**File:** `crates/contracts/src/lib.rs`

Tambahkan DTO baru:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SalesReportQuery {
    pub date_from: Option<String>,    // "2025-01-01"
    pub date_to: Option<String>,      // "2025-01-31"
    pub status_filter: Option<String>, // "delivered", "all"
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SalesReportRow {
    pub order_id: String,
    pub order_date: String,
    pub buyer_name: String,
    pub buyer_email: Option<String>,
    pub items: Vec<SalesReportItem>,
    pub subtotal_cents: i64,
    pub shipping_cents: i64,
    pub discount_cents: i64,
    pub total_cents: i64,
    pub status: String,
    pub payment_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SalesReportItem {
    pub product_name: String,
    pub quantity: i32,
    pub unit_price_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SalesReportSummary {
    pub total_revenue_cents: i64,
    pub total_orders: i32,
    pub average_order_value_cents: i64,
    pub total_items_sold: i32,
    pub orders_by_status: std::collections::HashMap<String, i32>,
}
```

### Langkah 2: Tambah Method di AnalyticsContract

**File:** `crates/contracts/src/lib.rs`

Cari `pub trait AnalyticsContract`, tambahkan:

```rust
async fn generate_sales_report(
    &self,
    query: SalesReportQuery,
) -> Result<(SalesReportSummary, Vec<SalesReportRow>), ContractError>;
```

### Langkah 3: Implementasi di AnalyticsModule

**File:** `crates/modules/analytics/src/lib.rs`

```rust
async fn generate_sales_report(
    &self,
    query: SalesReportQuery,
) -> Result<(SalesReportSummary, Vec<SalesReportRow>), ContractError> {
    let date_from = query.date_from.unwrap_or("1970-01-01".into());
    let date_to = query.date_to.unwrap_or("9999-12-31".into());
    
    // Query orders with JOIN to order_items, catalog, buyers
    // Filter by date range and optional status
    // Calculate summary aggregates
    // Return (summary, rows)
}
```

### Langkah 4: Tambah Handler HTTP

**File:** `crates/web/src/handlers/analytics.rs`

```rust
/// GET /api/v1/analytics/report?date_from=2025-01-01&date_to=2025-01-31
pub async fn get_sales_report_handler(
    State(state): State<AppState>,
    Query(query): Query<SalesReportQuery>,
) -> Result<Json<SalesReport>, ApiError> {
    let (summary, rows) = state.analytics_contract
        .generate_sales_report(query)
        .await
        .map_err(ApiError::from_contract)?;
    Ok(Json(SalesReport { summary, rows }))
}
```

### Langkah 5: Daftarkan Route

**File:** `crates/web/src/routes.rs`

Di `admin_routes`:

```rust
.route("/api/v1/analytics/report", get(get_sales_report_handler))
```

### Langkah 6: Buat UI Laporan di Admin Dashboard

**File:** `crates/web/static/admin/admin-analytics.js`

Tambahkan section report:

```javascript
function renderSalesReport() {
    return `
    <div class="report-filters">
        <label>Dari: <input type="date" id="reportDateFrom" /></label>
        <label>Sampai: <input type="date" id="reportDateTo" /></label>
        <label>Status:
            <select id="reportStatusFilter">
                <option value="">Semua</option>
                <option value="delivered">Delivered</option>
                <option value="paid">Paid</option>
            </select>
        </label>
        <button onclick="loadSalesReport()" class="btn-primary">Tampilkan</button>
        <button onclick="exportCSV()" class="btn-secondary">📥 Export CSV</button>
        <button onclick="window.print()" class="btn-secondary">🖨️ Cetak</button>
    </div>
    <div id="reportSummaryCards"></div>
    <table id="reportTable" class="report-table">
        <thead>
            <tr>
                <th>Tanggal</th>
                <th>Order ID</th>
                <th>Buyer</th>
                <th>Produk</th>
                <th>Qty</th>
                <th>Total</th>
                <th>Status</th>
            </tr>
        </thead>
        <tbody id="reportBody"></tbody>
    </table>`;
}
```

### Langkah 7: Implementasi Export CSV (Frontend)

```javascript
function exportCSV() {
    const rows = window.reportData?.rows || [];
    if (!rows.length) { alert('Tidak ada data untuk di-export'); return; }
    
    let csv = 'Tanggal,Order ID,Buyer,Produk,Qty,Total (Rp),Status\n';
    
    for (const row of rows) {
        const items = row.items.map(i => `${i.product_name} x${i.quantity}`).join('; ');
        csv += `"${row.order_date}","${row.order_id}","${row.buyer_name}","${items}","${row.items.reduce((s,i)=>s+i.quantity,0)}","${row.total_cents/100}","${row.status}"\n`;
    }
    
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = `laporan-penjualan-${new Date().toISOString().slice(0,10)}.csv`;
    link.click();
}
```

### Langkah 8: Tambah Print Styles

**File:** `crates/web/static/style.css`

```css
@media print {
    .sidebar, .nav-header, .report-filters button, .no-print { display: none !important; }
    .report-table { width: 100%; border-collapse: collapse; }
    .report-table th, .report-table td { border: 1px solid #000; padding: 4px 8px; font-size: 10pt; }
    .report-summary-card { border: 1px solid #000; padding: 8px; }
}
```

### Langkah 9: Tulis Unit Test

**File:** `crates/web/tests/analytics_report_test.rs`

```rust
#[tokio::test]
async fn test_sales_report_with_date_filter() {
    // 1. Create orders on different dates
    // 2. Query report with date range
    // 3. Verify only matching orders returned
}

#[tokio::test]
async fn test_sales_report_summary_calculation() {
    // 1. Create 3 orders with known totals
    // 2. Query report
    // 3. Verify summary: total_revenue, total_orders, average_order_value
}
```

### Langkah 10: Verifikasi

```bash
cargo test --workspace
cargo check --workspace
```

---

## ⚠️ Perhatian

- Endpoint report HARUS dilindungi `require_admin` middleware (bukan `require_auth`)
- Export CSV dilakukan di **frontend** (client-side) — tidak perlu endpoint khusus
- Gunakan format currency Indonesia: `Rp 1.234.567`
- Date format ISO 8601: `YYYY-MM-DD`
- Untuk dataset besar, pertimbangkan pagination (limit 1000 rows per query)
