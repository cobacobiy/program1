use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::message::{Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared trait for sending transactional emails across the modular monolith
#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String>;
}

/// Development & local fallback: logs email details without establishing network connection
#[derive(Debug, Default, Clone)]
pub struct ConsoleEmailSender;

impl ConsoleEmailSender {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EmailSender for ConsoleEmailSender {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String> {
        let trimmed_to = to.trim();
        if trimmed_to.is_empty() || !trimmed_to.contains('@') {
            return Err(format!("Invalid recipient email address: '{}'", to));
        }

        tracing::info!(
            target: "email",
            "[CONSOLE EMAIL] To: {} | Subject: '{}' | Body length: {} bytes",
            trimmed_to,
            subject,
            html_body.len()
        );
        Ok(())
    }
}

/// In-memory mock email sender for fast, deterministic unit and integration tests
#[derive(Debug, Clone, Default)]
pub struct MockEmailRecord {
    pub to: String,
    pub subject: String,
    pub html_body: String,
}

#[derive(Debug, Clone, Default)]
pub struct MockEmailSender {
    pub sent_emails: Arc<Mutex<Vec<MockEmailRecord>>>,
}

impl MockEmailSender {
    pub fn new() -> Self {
        Self {
            sent_emails: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn get_sent_emails(&self) -> Vec<MockEmailRecord> {
        self.sent_emails.lock().await.clone()
    }

    pub async fn clear(&self) {
        self.sent_emails.lock().await.clear();
    }
}

#[async_trait]
impl EmailSender for MockEmailSender {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String> {
        let trimmed_to = to.trim();
        if trimmed_to.is_empty() || !trimmed_to.contains('@') {
            return Err(format!("Invalid recipient email address: '{}'", to));
        }

        self.sent_emails.lock().await.push(MockEmailRecord {
            to: trimmed_to.to_string(),
            subject: subject.to_string(),
            html_body: html_body.to_string(),
        });
        Ok(())
    }
}

/// Configuration settings for SMTP email delivery
#[derive(Debug, Clone)]
pub struct SmtpEmailConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub from_name: String,
    pub from_email: String,
}

/// Production SMTP email sender backed by `lettre`
#[derive(Clone)]
pub struct SmtpEmailSender {
    config: SmtpEmailConfig,
}

impl SmtpEmailSender {
    pub fn new(config: SmtpEmailConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String> {
        let trimmed_to = to.trim();
        if trimmed_to.is_empty() || !trimmed_to.contains('@') {
            return Err(format!("Invalid recipient email address: '{}'", to));
        }

        let from_header = format!("{} <{}>", self.config.from_name, self.config.from_email);
        let from_mailbox: Mailbox = from_header
            .parse()
            .map_err(|e| format!("Invalid sender address: {}", e))?;
        let to_mailbox: Mailbox = trimmed_to
            .parse()
            .map_err(|e| format!("Invalid recipient address: {}", e))?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| format!("Failed to build email message: {}", e))?;

        let mut builder = if self.config.port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.host)
                .map_err(|e| format!("SMTP relay configuration error: {}", e))?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.host)
                .map_err(|e| format!("SMTP starttls relay error: {}", e))?
                .port(self.config.port)
        };

        if let Some(ref pass) = self.config.password {
            builder = builder.credentials(Credentials::new(self.config.username.clone(), pass.clone()));
        }

        let transport = builder.build();
        transport
            .send(email)
            .await
            .map_err(|e| format!("SMTP send failed: {}", e))?;

        Ok(())
    }
}

// ============================================================================
// EMAIL TEMPLATES (RESPONSIVE INLINE CSS FOR MAXIMUM CLIENT COMPATIBILITY)
// ============================================================================

/// Generates Welcome Email (Subject, HTML Body)
pub fn welcome_email(buyer_name: &str, store_name: &str, login_url: &str) -> (String, String) {
    let subject = format!("Selamat Datang di {} — Akun Anda Siap Digunakan!", store_name);
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{subject}</title>
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; background-color: #f4f5f7; color: #1f2937;">
  <table width="100%" border="0" cellspacing="0" cellpadding="0" style="background-color: #f4f5f7; padding: 30px 15px;">
    <tr>
      <td align="center">
        <table width="100%" border="0" cellspacing="0" cellpadding="0" style="max-width: 580px; background-color: #ffffff; border-radius: 12px; overflow: hidden; box-shadow: 0 4px 12px rgba(0,0,0,0.06);">
          <!-- Header -->
          <tr>
            <td style="background: linear-gradient(135deg, #ee4d2d 0%, #ff7337 100%); padding: 32px 24px; text-align: center;">
              <h1 style="color: #ffffff; margin: 0; font-size: 24px; font-weight: 700; letter-spacing: -0.5px;">{store_name}</h1>
              <p style="color: #fff2ed; margin: 8px 0 0 0; font-size: 14px;">Selamat bergabung sebagai pelanggan istimewa!</p>
            </td>
          </tr>
          <!-- Body -->
          <tr>
            <td style="padding: 32px 28px;">
              <h2 style="margin: 0 0 16px 0; font-size: 18px; color: #111827;">Halo, {buyer_name}! 👋</h2>
              <p style="margin: 0 0 16px 0; font-size: 14px; line-height: 1.6; color: #4b5563;">
                Terima kasih telah mendaftar di <strong>{store_name}</strong>. Akun belanja Anda telah berhasil dibuat dan langsung aktif. Anda kini dapat berbelanja ribuan produk pilihan, menikmati penawaran eksklusif, serta memantau status pesanan secara real-time.
              </p>
              <div style="text-align: center; margin: 32px 0;">
                <a href="{login_url}" style="display: inline-block; background-color: #ee4d2d; color: #ffffff; text-decoration: none; padding: 14px 32px; border-radius: 8px; font-weight: 600; font-size: 15px; box-shadow: 0 2px 6px rgba(238, 77, 45, 0.35);">
                  Mulai Belanja Sekarang &rarr;
                </a>
              </div>
              <p style="margin: 0; font-size: 13px; line-height: 1.5; color: #6b7280; border-top: 1px solid #e5e7eb; padding-top: 20px;">
                Jika Anda merasa tidak pernah mendaftar di {store_name}, Anda dapat mengabaikan email ini dengan aman.
              </p>
            </td>
          </tr>
          <!-- Footer -->
          <tr>
            <td style="background-color: #f9fafb; padding: 20px 24px; text-align: center; border-top: 1px solid #f3f4f6;">
              <p style="margin: 0; font-size: 12px; color: #9ca3af;">
                &copy; {store_name}. Seluruh hak cipta dilindungi undang-undang.
              </p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#
    );
    (subject, html)
}

/// Item summary for order confirmation email
#[derive(Debug, Clone)]
pub struct OrderEmailItem {
    pub name: String,
    pub quantity: u32,
    pub price: f64,
}

/// Helper to format numbers into standard Indonesian Rupiah currency string (e.g. "Rp 150.000")
pub fn format_idr(amount: f64) -> String {
    let int_part = amount.round().abs() as i64;
    let s = int_part.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('.');
        }
        result.push(c);
    }
    if amount < 0.0 {
        format!("-Rp {}", result)
    } else {
        format!("Rp {}", result)
    }
}

/// Generates Order Confirmation Email (Subject, HTML Body)
pub fn order_confirmation_email(
    order_id: &str,
    customer_name: &str,
    total_amount: f64,
    items: &[OrderEmailItem],
    shipping_address: &str,
    store_name: &str,
) -> (String, String) {
    let short_id = if order_id.len() >= 8 {
        &order_id[..8]
    } else {
        order_id
    };
    let subject = format!("Konfirmasi Pesanan #{} — {}", short_id, store_name);
    let total_str = format_idr(total_amount);

    let mut items_rows = String::new();
    for it in items {
        let subtotal = it.price * (it.quantity as f64);
        let subtotal_str = format_idr(subtotal);
        items_rows.push_str(&format!(
            r#"<tr>
                <td style="padding: 10px 0; border-bottom: 1px solid #f3f4f6; font-size: 14px; color: #374151;">{} &times; {}</td>
                <td style="padding: 10px 0; border-bottom: 1px solid #f3f4f6; font-size: 14px; color: #111827; text-align: right; font-weight: 600;">{}</td>
              </tr>"#,
            it.name, it.quantity, subtotal_str
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{subject}</title>
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; background-color: #f4f5f7; color: #1f2937;">
  <table width="100%" border="0" cellspacing="0" cellpadding="0" style="background-color: #f4f5f7; padding: 30px 15px;">
    <tr>
      <td align="center">
        <table width="100%" border="0" cellspacing="0" cellpadding="0" style="max-width: 580px; background-color: #ffffff; border-radius: 12px; overflow: hidden; box-shadow: 0 4px 12px rgba(0,0,0,0.06);">
          <!-- Header -->
          <tr>
            <td style="background: linear-gradient(135deg, #ee4d2d 0%, #ff7337 100%); padding: 32px 24px; text-align: center;">
              <h1 style="color: #ffffff; margin: 0; font-size: 24px; font-weight: 700;">Pesanan Berhasil Diterima</h1>
              <p style="color: #fff2ed; margin: 8px 0 0 0; font-size: 14px;">Nomor Pesanan: #{short_id}</p>
            </td>
          </tr>
          <!-- Content -->
          <tr>
            <td style="padding: 32px 28px;">
              <p style="margin: 0 0 16px 0; font-size: 14px; color: #4b5563;">
                Halo <strong>{customer_name}</strong>, pesanan Anda telah berhasil dibuat di <strong>{store_name}</strong>. Tim kami sedang memproses pesanan Anda.
              </p>
              <!-- Order Items Table -->
              <table width="100%" border="0" cellspacing="0" cellpadding="0" style="margin: 20px 0;">
                <thead>
                  <tr>
                    <th style="padding-bottom: 8px; border-bottom: 2px solid #e5e7eb; font-size: 13px; color: #6b7280; text-align: left; text-transform: uppercase;">Produk</th>
                    <th style="padding-bottom: 8px; border-bottom: 2px solid #e5e7eb; font-size: 13px; color: #6b7280; text-align: right; text-transform: uppercase;">Total</th>
                  </tr>
                </thead>
                <tbody>
                  {items_rows}
                  <tr>
                    <td style="padding: 14px 0 0 0; font-size: 15px; font-weight: 700; color: #111827;">Total Pembayaran</td>
                    <td style="padding: 14px 0 0 0; font-size: 17px; font-weight: 700; color: #ee4d2d; text-align: right;">{total_str}</td>
                  </tr>
                </tbody>
              </table>

              <!-- Shipping Address Box -->
              <div style="background-color: #f9fafb; border-radius: 8px; padding: 16px; margin: 24px 0 16px 0; border: 1px solid #e5e7eb;">
                <h3 style="margin: 0 0 6px 0; font-size: 13px; text-transform: uppercase; color: #6b7280;">Alamat Pengiriman:</h3>
                <p style="margin: 0; font-size: 14px; color: #374151; line-height: 1.5;">{shipping_address}</p>
              </div>
            </td>
          </tr>
          <!-- Footer -->
          <tr>
            <td style="background-color: #f9fafb; padding: 20px 24px; text-align: center; border-top: 1px solid #f3f4f6;">
              <p style="margin: 0; font-size: 12px; color: #9ca3af;">
                &copy; {store_name}. Terima kasih atas kepercayaan Anda berbelanja bersama kami!
              </p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#
    );
    (subject, html)
}

/// Generates Payment Success Email (Subject, HTML Body)
pub fn payment_success_email(
    order_id: &str,
    customer_name: &str,
    payment_method: &str,
    amount: f64,
    store_name: &str,
) -> (String, String) {
    let short_id = if order_id.len() >= 8 {
        &order_id[..8]
    } else {
        order_id
    };
    let subject = format!("Pembayaran Berhasil untuk Pesanan #{} — {}", short_id, store_name);
    let amount_str = format_idr(amount);
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{subject}</title>
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; background-color: #f4f5f7; color: #1f2937;">
  <table width="100%" border="0" cellspacing="0" cellpadding="0" style="background-color: #f4f5f7; padding: 30px 15px;">
    <tr>
      <td align="center">
        <table width="100%" border="0" cellspacing="0" cellpadding="0" style="max-width: 580px; background-color: #ffffff; border-radius: 12px; overflow: hidden; box-shadow: 0 4px 12px rgba(0,0,0,0.06);">
          <tr>
            <td style="background: linear-gradient(135deg, #10b981 0%, #059669 100%); padding: 32px 24px; text-align: center;">
              <div style="font-size: 40px; line-height: 1; margin-bottom: 8px;">✅</div>
              <h1 style="color: #ffffff; margin: 0; font-size: 24px; font-weight: 700;">Pembayaran Berhasil Dikonfirmasi</h1>
              <p style="color: #d1fae5; margin: 8px 0 0 0; font-size: 14px;">Pesanan #{short_id}</p>
            </td>
          </tr>
          <tr>
            <td style="padding: 32px 28px;">
              <p style="margin: 0 0 16px 0; font-size: 14px; color: #4b5563;">
                Halo <strong>{customer_name}</strong>, pembayaran untuk pesanan Anda telah kami terima secara lunas.
              </p>
              <div style="background-color: #ecfdf5; border-radius: 8px; padding: 18px; margin: 20px 0; border: 1px solid #a7f3d0;">
                <table width="100%" border="0" cellspacing="0" cellpadding="0">
                  <tr>
                    <td style="padding: 4px 0; font-size: 13px; color: #065f46;">Metode Pembayaran:</td>
                    <td style="padding: 4px 0; font-size: 13px; font-weight: 600; color: #065f46; text-align: right;">{payment_method}</td>
                  </tr>
                  <tr>
                    <td style="padding: 4px 0; font-size: 13px; color: #065f46;">Jumlah Dibayar:</td>
                    <td style="padding: 4px 0; font-size: 15px; font-weight: 700; color: #065f46; text-align: right;">{amount_str}</td>
                  </tr>
                  <tr>
                    <td style="padding: 4px 0; font-size: 13px; color: #065f46;">Status Pesanan:</td>
                    <td style="padding: 4px 0; font-size: 13px; font-weight: 600; color: #059669; text-align: right;">Sedang Dikemas / Diproses</td>
                  </tr>
                </table>
              </div>
              <p style="margin: 0; font-size: 14px; line-height: 1.6; color: #4b5563;">
                Pesanan Anda segera disiapkan oleh bagian gudang. Kami akan mengirimkan notifikasi beserta nomor resi pengiriman segera setelah paket diserahkan ke pihak kurir.
              </p>
            </td>
          </tr>
          <tr>
            <td style="background-color: #f9fafb; padding: 20px 24px; text-align: center; border-top: 1px solid #f3f4f6;">
              <p style="margin: 0; font-size: 12px; color: #9ca3af;">
                &copy; {store_name}. Terima kasih atas kerja sama dan kepercayaan Anda!
              </p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#
    );
    (subject, html)
}

/// Generates Shipping Notification Email (Subject, HTML Body)
pub fn shipping_notification_email(
    order_id: &str,
    customer_name: &str,
    tracking_number: Option<&str>,
    courier: Option<&str>,
    store_name: &str,
) -> (String, String) {
    let short_id = if order_id.len() >= 8 {
        &order_id[..8]
    } else {
        order_id
    };
    let subject = format!("Pesanan #{} Sedang Dalam Pengiriman! — {}", short_id, store_name);
    let tracking_display = tracking_number.unwrap_or("Belum Tersedia / Menunggu Update Kurir");
    let courier_display = courier.unwrap_or("Kurir Standar");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{subject}</title>
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; background-color: #f4f5f7; color: #1f2937;">
  <table width="100%" border="0" cellspacing="0" cellpadding="0" style="background-color: #f4f5f7; padding: 30px 15px;">
    <tr>
      <td align="center">
        <table width="100%" border="0" cellspacing="0" cellpadding="0" style="max-width: 580px; background-color: #ffffff; border-radius: 12px; overflow: hidden; box-shadow: 0 4px 12px rgba(0,0,0,0.06);">
          <tr>
            <td style="background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%); padding: 32px 24px; text-align: center;">
              <div style="font-size: 40px; line-height: 1; margin-bottom: 8px;">🚚</div>
              <h1 style="color: #ffffff; margin: 0; font-size: 24px; font-weight: 700;">Pesanan Anda Sedang Dikirim</h1>
              <p style="color: #dbeafe; margin: 8px 0 0 0; font-size: 14px;">Nomor Pesanan: #{short_id}</p>
            </td>
          </tr>
          <tr>
            <td style="padding: 32px 28px;">
              <p style="margin: 0 0 16px 0; font-size: 14px; color: #4b5563;">
                Halo <strong>{customer_name}</strong>, paket pesanan Anda telah diserahkan kepada kurir dan saat ini sedang dalam perjalanan menuju alamat Anda.
              </p>
              <div style="background-color: #eff6ff; border-radius: 8px; padding: 18px; margin: 20px 0; border: 1px solid #bfdbfe;">
                <table width="100%" border="0" cellspacing="0" cellpadding="0">
                  <tr>
                    <td style="padding: 4px 0; font-size: 13px; color: #1e40af;">Jasa Ekspedisi:</td>
                    <td style="padding: 4px 0; font-size: 13px; font-weight: 600; color: #1e40af; text-align: right;">{courier_display}</td>
                  </tr>
                  <tr>
                    <td style="padding: 4px 0; font-size: 13px; color: #1e40af;">Nomor Resi / AWB:</td>
                    <td style="padding: 4px 0; font-size: 14px; font-family: monospace; font-weight: 700; color: #1e3a8a; text-align: right;">{tracking_display}</td>
                  </tr>
                </table>
              </div>
              <p style="margin: 0; font-size: 14px; line-height: 1.6; color: #4b5563;">
                Anda dapat memantau status pengiriman paket ini sewaktu-waktu di dashboard profil akun toko Anda. Harap pastikan nomor telepon Anda dapat dihubungi oleh pihak kurir saat paket diantar.
              </p>
            </td>
          </tr>
          <tr>
            <td style="background-color: #f9fafb; padding: 20px 24px; text-align: center; border-top: 1px solid #f3f4f6;">
              <p style="margin: 0; font-size: 12px; color: #9ca3af;">
                &copy; {store_name}. Terima kasih atas kesabaran dan kerja sama Anda!
              </p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#
    );
    (subject, html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_console_email_sender_succeeds() {
        let sender = ConsoleEmailSender::new();
        let res = sender
            .send_email(
                "buyer@example.com",
                "Test Subject",
                "<p>Hello World</p>",
            )
            .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_console_email_sender_rejects_invalid_email() {
        let sender = ConsoleEmailSender::new();
        let res = sender
            .send_email("invalid-email", "Test Subject", "<p>Hello</p>")
            .await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid recipient email"));

        let empty_res = sender.send_email("", "Test", "<p>Test</p>").await;
        assert!(empty_res.is_err());
    }

    #[tokio::test]
    async fn test_mock_email_sender_records() {
        let mock = MockEmailSender::new();
        mock.send_email("alice@domain.com", "Sub 1", "Body 1")
            .await
            .unwrap();
        mock.send_email("bob@domain.com", "Sub 2", "Body 2")
            .await
            .unwrap();

        let emails = mock.get_sent_emails().await;
        assert_eq!(emails.len(), 2);
        assert_eq!(emails[0].to, "alice@domain.com");
        assert_eq!(emails[0].subject, "Sub 1");
        assert_eq!(emails[1].to, "bob@domain.com");

        mock.clear().await;
        assert!(mock.get_sent_emails().await.is_empty());
    }

    #[test]
    fn test_email_templates_content() {
        // 1. Welcome Email
        let (w_sub, w_html) = welcome_email("Budi Santoso", "AURA Store", "http://localhost:6090");
        assert!(w_sub.contains("AURA Store"));
        assert!(w_html.contains("Budi Santoso"));
        assert!(w_html.contains("http://localhost:6090"));

        // 2. Order Confirmation Email
        let items = vec![OrderEmailItem {
            name: "Sepatu Sneaker".to_string(),
            quantity: 2,
            price: 150_000.0,
        }];
        let (o_sub, o_html) = order_confirmation_email(
            "order-12345678-abcd",
            "Siti Rahma",
            300_000.0,
            &items,
            "Jl. Mawar No. 10, Jakarta",
            "AURA Store",
        );
        assert!(o_sub.contains("order-12"));
        assert!(o_html.contains("Sepatu Sneaker"));
        assert!(o_html.contains("Rp 300.000"));
        assert!(o_html.contains("Jl. Mawar No. 10"));

        // 3. Payment Success Email
        let (p_sub, p_html) = payment_success_email(
            "order-12345678-abcd",
            "Siti Rahma",
            "Midtrans QRIS / GoPay",
            300_000.0,
            "AURA Store",
        );
        assert!(p_sub.contains("Pembayaran Berhasil"));
        assert!(p_html.contains("Midtrans QRIS / GoPay"));
        assert!(p_html.contains("Rp 300.000"));

        // 4. Shipping Notification Email
        let (s_sub, s_html) = shipping_notification_email(
            "order-12345678-abcd",
            "Siti Rahma",
            Some("JNE-AWB-998877"),
            Some("JNE Express"),
            "AURA Store",
        );
        assert!(s_sub.contains("Sedang Dalam Pengiriman"));
        assert!(s_html.contains("JNE-AWB-998877"));
        assert!(s_html.contains("JNE Express"));
    }
}
