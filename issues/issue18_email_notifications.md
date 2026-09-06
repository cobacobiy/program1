# Issue #18 — Email Notification System (Order Confirmation & OTP)

> **Prioritas**: 🟢 MEDIUM
> **Estimasi**: 2-3 hari
> **Depends On**: Issue #14 (Payment Gateway)
> **Skill Level**: Junior Rust Developer

---

## 🔍 Masalah Saat Ini

Tidak ada notifikasi email sama sekali. Buyer yang checkout atau bayar tidak mendapat konfirmasi apapun selain response di browser.

Yang seharusnya dikirim:
- ❌ Email konfirmasi order setelah checkout
- ❌ Email konfirmasi pembayaran
- ❌ Email status update (shipped, delivered)
- ❌ Email verifikasi akun saat registrasi buyer
- ❌ Email OTP sebagai alternatif WhatsApp OTP

---

## ✅ Acceptance Criteria

### Step 1: Definisikan `EmailSender` Trait

Buat di `crates/modules/buyer/src/lib.rs` (atau buat crate `crates/core/src/email.rs`):

```rust
#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String>;
}
```

### Step 2: Implementasi

**Dev mode:** `ConsoleEmailSender` — log ke stdout (tidak kirim email betulan)

**Production:** `SmtpEmailSender` — kirim email via SMTP (menggunakan `lettre` crate)

```toml
[dependencies]
lettre = { version = "0.11", features = ["tokio1-native-tls", "builder", "hostname"] }
```

### Step 3: Environment Variables

Tambahkan ke `.env.example`:
```env
# --- Email (SMTP) ---
EMAIL_PROVIDER=console             # console | smtp
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=noreply@yourdomain.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_NAME="AURA Storefront"
SMTP_FROM_EMAIL=noreply@yourdomain.com
```

### Step 4: Email Templates (Hardcoded HTML)

Buat helper functions untuk setiap template:

| Template | Trigger | Data |
|----------|---------|------|
| `order_confirmation_email(order, buyer)` | Order created | Order ID, items, total, shipping address |
| `payment_success_email(order, payment)` | Payment confirmed | Order ID, payment method, amount |
| `shipping_notification_email(order, tracking)` | Order shipped | Order ID, tracking number, courier |
| `welcome_email(buyer)` | Buyer register | Buyer name, login link |

Template HTML sederhana — inline CSS, responsive, clean.

### Step 5: Integrasikan ke Order & Buyer Flow

1. **Buyer Register** → kirim `welcome_email`
2. **Order Created** → kirim `order_confirmation_email`
3. **Payment Confirmed** → kirim `payment_success_email`
4. **Order Shipped** → kirim `shipping_notification_email`

Kirim email secara **async** (spawn tokio task) agar tidak blocking response.

### Step 6: Unit Tests

1. Test `ConsoleEmailSender` — logs to stdout, returns Ok
2. Test email template rendering — correct HTML output
3. Test invalid email address handling

---

## 📁 File Yang Harus Dibuat/Diubah

| Action | File |
|--------|------|
| **CREATE** | `crates/core/src/email.rs` (EmailSender trait + implementations) |
| **MODIFY** | `crates/core/src/lib.rs` (export email module) |
| **MODIFY** | `crates/core/Cargo.toml` (add lettre dependency) |
| **MODIFY** | `crates/core/src/config.rs` (add email config) |
| **MODIFY** | `.env.example` (add email variables) |
| **MODIFY** | `crates/modules/buyer/src/lib.rs` (send welcome email on register) |
| **MODIFY** | `crates/modules/order/src/lib.rs` (send order email) |
| **MODIFY** | `crates/web/src/main.rs` (init EmailSender) |

---

## ⚠️ Catatan Penting

- Gunakan `ConsoleEmailSender` di development, `SmtpEmailSender` di production
- **Jangan blocking** — spawn email sending di background task
- SMTP credentials adalah **secret** — JANGAN hardcode
- Email HTML harus inline CSS (email client tidak support external CSS)
- Pastikan `cargo test --workspace` pass sebelum push
