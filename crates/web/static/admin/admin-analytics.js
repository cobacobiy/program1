/* ==========================================================================
   ADMIN ANALYTICS & CHANNELS ENGINE
   File: /crates/web/static/admin/admin-analytics.js
   ========================================================================== */

function getBadgeClass(ch) {
  if (ch === "TikTokShop" || ch === "tiktok") return "badge-tiktok";
  if (ch === "Shopee" || ch === "shopee") return "badge-shopee";
  if (ch === "Tokopedia" || ch === "tokopedia") return "badge-tokopedia";
  return "badge-native";
}

function renderAnalyticsBreakdown(breakdown) {
  const container = document.getElementById("analytics-breakdown-container");
  if (!container) return;

  container.innerHTML = `
    <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(220px, 1fr)); gap:1.2rem">
      ${breakdown.map(b => `
        <div style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1.2rem; border-radius:12px">
          <span class="channel-badge ${getBadgeClass(b.channel)}">${escapeHtml(b.channel_name)}</span>
          <div style="font-size:1.4rem; font-weight:700; margin-top:0.6rem; color:var(--emerald)">Rp ${b.total_revenue.toLocaleString("id-ID")}</div>
          <div style="font-size:0.8rem; color:var(--text-muted); margin-top:0.2rem">Total Orders: <strong>${b.total_orders}</strong></div>
        </div>
      `).join("")}
    </div>
  `;
}

function renderChannelsGrid(channels) {
  const dashGrid = document.getElementById("dash-channels-grid");
  const html = channels.map(c => `
    <div style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1.2rem; border-radius:12px; display:flex; flex-direction:column; justify-content:space-between">
      <div>
        <div style="display:flex; justify-content:space-between; align-items:center">
          <span class="channel-badge ${getBadgeClass(c.channel)}">${escapeHtml(c.name)}</span>
          <span style="font-size:0.75rem; color:${c.is_connected ? "var(--emerald)" : "var(--rose)"}">
            ${c.is_connected ? "● Connected" : "○ Disconnected"}
          </span>
        </div>
        <div style="font-size:1.3rem; font-weight:700; margin-top:0.8rem; color:#fff">${c.active_products_synced} Products</div>
        <div style="font-size:0.75rem; color:var(--text-muted); margin-top:0.2rem">Last Sync: ${new Date(c.last_synced_at).toLocaleTimeString()}</div>
      </div>
      <button class="btn-sm" style="margin-top:1rem; width:100%" onclick="syncChannel('${c.channel.toLowerCase()}')">⚡ Sync Now</button>
    </div>
  `).join("");

  if (dashGrid) dashGrid.innerHTML = html;
  const pageGrid = document.getElementById("channels-page-grid");
  if (pageGrid) pageGrid.innerHTML = html;
}

// Window Exports
window.getBadgeClass = getBadgeClass;
window.renderAnalyticsBreakdown = renderAnalyticsBreakdown;
window.renderChannelsGrid = renderChannelsGrid;
