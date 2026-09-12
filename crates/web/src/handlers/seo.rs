use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use uuid::Uuid;

use crate::state::AppState;

/// Robots.txt endpoint
pub async fn get_robots_txt() -> impl IntoResponse {
    let content = "User-agent: *\nAllow: /\nDisallow: /admin\nDisallow: /api/\n\nSitemap: /sitemap.xml\n";
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], content)
}

/// Dynamic XML Sitemap endpoint
pub async fn get_sitemap_xml(
    State(state): State<AppState>,
) -> Result<Response, (StatusCode, String)> {
    let items = state
        .catalog_contract
        .list_items()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let categories = state
        .catalog_contract
        .list_categories()
        .await
        .unwrap_or_default();

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

    // Homepage
    xml.push_str("  <url>\n");
    xml.push_str("    <loc>/</loc>\n");
    xml.push_str("    <changefreq>daily</changefreq>\n");
    xml.push_str("    <priority>1.0</priority>\n");
    xml.push_str("  </url>\n");

    // Products
    for item in items {
        let lastmod = item.created_at.to_rfc3339();
        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>/product/{}</loc>\n", item.id));
        xml.push_str(&format!("    <lastmod>{}</lastmod>\n", lastmod));
        xml.push_str("    <changefreq>weekly</changefreq>\n");
        xml.push_str("    <priority>0.8</priority>\n");
        xml.push_str("  </url>\n");
    }

    // Categories
    for cat in categories {
        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>/#category-{}</loc>\n", cat.slug));
        xml.push_str("    <changefreq>weekly</changefreq>\n");
        xml.push_str("    <priority>0.6</priority>\n");
        xml.push_str("  </url>\n");
    }

    xml.push_str("</urlset>\n");

    Ok((
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response())
}

/// Helper to check if a user-agent belongs to a social media or search crawler bot
pub fn is_social_crawler(user_agent: &str) -> bool {
    let ua = user_agent.to_lowercase();
    ua.contains("whatsapp")
        || ua.contains("facebookexternalhit")
        || ua.contains("twitterbot")
        || ua.contains("telegrambot")
        || ua.contains("googlebot")
        || ua.contains("bingbot")
        || ua.contains("slackbot")
        || ua.contains("linkedinbot")
        || ua.contains("discordbot")
        || ua.contains("pinterest")
        || ua.contains("crawler")
        || ua.contains("spider")
}

/// Product page handler: returns Open Graph & Twitter metadata if crawler, or redirects to SPA product view
pub async fn get_product_page_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let item = state
        .catalog_contract
        .get_item(id)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Product not found".to_string()))?;

    if is_social_crawler(user_agent) {
        let title = html_escape(&format!("{} | {}", item.name, state.store_name));
        let description = html_escape(&item.description);
        let image = if item.image_url.is_empty() {
            "https://via.placeholder.com/600x400?text=No+Image"
        } else {
            &item.image_url
        };
        let price = format!("{:.0}", item.price);
        let url = format!("/product/{}", item.id);
        let spa_target = format!("/#product-{}", item.id);

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="id">
<head>
  <meta charset="UTF-8">
  <title>{title}</title>
  <meta name="description" content="{description}">

  <!-- Open Graph / Facebook / WhatsApp -->
  <meta property="og:type" content="product">
  <meta property="og:site_name" content="{store_name}">
  <meta property="og:title" content="{title}">
  <meta property="og:description" content="{description}">
  <meta property="og:image" content="{image}">
  <meta property="og:url" content="{url}">
  <meta property="product:price:amount" content="{price}">
  <meta property="product:price:currency" content="{currency}">

  <!-- Twitter Card -->
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="{title}">
  <meta name="twitter:description" content="{description}">
  <meta name="twitter:image" content="{image}">

  <!-- Human redirect fallback -->
  <meta http-equiv="refresh" content="0; url={spa_target}">
</head>
<body>
  <h1>{title}</h1>
  <p>{description}</p>
  <p>Harga: {currency} {price}</p>
  <script>window.location.replace('{spa_target}');</script>
</body>
</html>"#,
            title = title,
            description = description,
            store_name = html_escape(&state.store_name),
            image = html_escape(image),
            price = price,
            currency = html_escape(&state.store_currency),
            url = url,
            spa_target = spa_target,
        );

        Ok(Html(html).into_response())
    } else {
        // Redirect normal browsers to SPA with product hash
        let redirect_target = format!("/#product-{}", item.id);
        Ok((
            StatusCode::FOUND,
            [(header::LOCATION, redirect_target)],
            Body::empty(),
        )
            .into_response())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
