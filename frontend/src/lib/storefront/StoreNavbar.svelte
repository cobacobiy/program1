<script lang="ts">
  import type { HealthResponse } from '../types';
  import { auth } from '../auth.svelte';
  import { cart } from '../cart.svelte';
  import { i18n } from '../i18n.svelte';
  import NotificationBell from '../NotificationBell.svelte';

  interface Props {
    activeTab: 'store' | 'admin' | 'diagnostic';
    healthData: HealthResponse | null;
    wishlistCount: number;
    cartBounceTrigger: boolean;
    onSelectTab: (tab: 'store' | 'admin' | 'diagnostic') => void;
    onOpenWishlist: () => void;
    onOpenOrders: () => void;
    onOpenCart: () => void;
    onNotificationClick: (item: any) => void;
  }

  let {
    activeTab,
    healthData,
    wishlistCount,
    cartBounceTrigger,
    onSelectTab,
    onOpenWishlist,
    onOpenOrders,
    onOpenCart,
    onNotificationClick,
  }: Props = $props();
</script>

<header class="navbar">
  <div class="nav-inner">
    <div class="brand">
      <span class="logo-emoji">⚡</span>
      <div>
        <h2>Program1</h2>
        <span class="subtext">Svelte 5 + Rust Axum</span>
      </div>
    </div>

    <nav class="nav-tabs">
      <button class:active={activeTab === 'store'} onclick={() => onSelectTab('store')}>
        🏪 {i18n.t('nav.products', 'Katalog Toko')}
      </button>
      <button class:active={activeTab === 'admin'} onclick={() => onSelectTab('admin')}>
        ⚙️ {i18n.t('nav.admin', 'Admin Hub')}
      </button>
      <button class:active={activeTab === 'diagnostic'} onclick={() => onSelectTab('diagnostic')}>
        🔍 {i18n.t('nav.diagnostics', 'Server Health')}
        {#if healthData}
          <span class="status-dot green"></span>
        {:else}
          <span class="status-dot yellow"></span>
        {/if}
      </button>
    </nav>

    <div class="nav-user-actions">
      <!-- Language Switcher -->
      <div class="lang-switch-pills" role="group" aria-label={i18n.t('common.language', 'Pilih Bahasa')}>
        <button
          class="lang-pill"
          class:active={i18n.current === 'id'}
          onclick={() => i18n.setLanguage('id')}
          title="Bahasa Indonesia"
          type="button"
        >
          🇮🇩 ID
        </button>
        <button
          class="lang-pill"
          class:active={i18n.current === 'en'}
          onclick={() => i18n.setLanguage('en')}
          title="English"
          type="button"
        >
          🇬🇧 EN
        </button>
      </div>

      <NotificationBell role="buyer" onNotificationClick={onNotificationClick} />

      <button class="btn-wishlist" onclick={onOpenWishlist} title={i18n.t('nav.wishlist', 'Wishlist Saya')}>
        ❤️ {i18n.t('nav.wishlist', 'Wishlist')}
        {#if wishlistCount > 0}
          <span class="wishlist-count">{wishlistCount}</span>
        {/if}
      </button>

      {#if auth.user}
        <button class="btn-orders" onclick={onOpenOrders}>
          📦 {i18n.t('nav.orders', 'Pesanan Saya')}
        </button>
        <div class="user-pill">
          <div class="user-badge-col">
            <span class="uname">👤 {auth.user.name}</span>
            <div class="loyalty-chips-mini">
              <span class="user-tier-badge">👑 {auth.user.membership_tier || 'Classic'}</span>
              {#if auth.user.points_balance !== undefined}
                <span class="user-points-badge">⭐ {auth.user.points_balance.toLocaleString('id-ID')} Poin</span>
              {/if}
            </div>
          </div>
          <button class="btn-logout" onclick={() => auth.logout()}>{i18n.t('nav.logout', 'Keluar')}</button>
        </div>
      {:else}
        <button class="btn-login" onclick={() => auth.isModalOpen = true}>
          {i18n.t('auth.login_title', 'Masuk')} / {i18n.t('auth.register_title', 'Daftar')}
        </button>
      {/if}

      <button class="cart-trigger" class:cart-bump={cartBounceTrigger} onclick={onOpenCart}>
        🛒 {i18n.t('nav.cart', 'Keranjang')}
        {#if cart.totalItems > 0}
          <span class="cart-count" class:count-pop={cartBounceTrigger}>{cart.totalItems}</span>
        {/if}
      </button>
    </div>
  </div>
</header>

<style>
  .navbar {
    background: #0f172a;
    border-bottom: 1px solid #1e293b;
    position: sticky;
    top: 0;
    z-index: 100;
  }
  .nav-inner {
    max-width: 1200px;
    margin: 0 auto;
    padding: 0.75rem 1.5rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .brand { display: flex; align-items: center; gap: 0.6rem; }
  .logo-emoji { font-size: 1.7rem; }
  .brand h2 { margin: 0; font-size: 1.25rem; color: #38bdf8; }
  .subtext { font-size: 0.75rem; color: #94a3b8; }

  .nav-tabs { display: flex; gap: 0.5rem; }
  .nav-tabs button {
    background: transparent; border: none; color: #94a3b8;
    padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer;
    font-size: 0.9rem; display: flex; align-items: center; gap: 0.4rem;
  }
  .nav-tabs button:hover { background: #1e293b; color: #fff; }
  .nav-tabs button.active { background: #1e293b; color: #38bdf8; font-weight: bold; }
  .status-dot { width: 8px; height: 8px; border-radius: 50%; }
  .status-dot.green { background: #10b981; box-shadow: 0 0 6px #10b981; }
  .status-dot.yellow { background: #f59e0b; }

  .nav-user-actions { display: flex; align-items: center; gap: 0.75rem; }
  .lang-switch-pills {
    display: flex;
    align-items: center;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 20px;
    padding: 2px;
    gap: 2px;
  }
  .lang-pill {
    background: transparent;
    color: #94a3b8;
    border: none;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.25rem 0.6rem;
    border-radius: 16px;
    cursor: pointer;
    transition: all 0.2s ease;
  }
  .lang-pill:hover {
    color: #fff;
  }
  .lang-pill.active {
    background: #0284c7;
    color: #fff;
    box-shadow: 0 1px 4px rgba(2, 132, 199, 0.4);
  }
  .user-pill {
    display: flex; align-items: center; gap: 0.6rem;
    background: #1e293b; padding: 0.35rem 0.75rem; border-radius: 20px; border: 1px solid #334155;
  }
  .user-badge-col {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .uname { font-size: 0.85rem; color: #cbd5e1; line-height: 1.1; }
  .loyalty-chips-mini {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 0.7rem;
  }
  .user-tier-badge {
    color: #fde047;
    font-weight: 700;
  }
  .user-points-badge {
    color: #34d399;
    font-weight: 600;
  }
  .btn-logout {
    background: #475569; color: #fff; border: none; font-size: 0.75rem;
    padding: 0.2rem 0.5rem; border-radius: 4px; cursor: pointer;
  }
  .btn-login {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 0.9rem;
    border-radius: 6px; font-size: 0.9rem; font-weight: 500; cursor: pointer;
  }
  .btn-wishlist {
    background: #1e293b; color: #f8fafc; border: 1px solid #334155;
    padding: 0.5rem 0.85rem; border-radius: 6px; font-size: 0.85rem; font-weight: 600;
    cursor: pointer; display: flex; align-items: center; gap: 0.4rem; transition: all 0.2s;
  }
  .btn-wishlist:hover {
    background: #334155; border-color: #ef4444; color: #fca5a5;
  }
  .wishlist-count {
    background: #ef4444; color: #fff; font-size: 0.7rem; font-weight: bold;
    padding: 0.1rem 0.45rem; border-radius: 9999px;
  }
  .btn-orders {
    background: #1e293b; color: #38bdf8; border: 1px solid #334155;
    padding: 0.5rem 0.8rem; border-radius: 6px; font-size: 0.85rem; cursor: pointer;
  }
  .cart-trigger {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 1rem;
    border-radius: 6px; font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 0.4rem;
    transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.2s;
  }
  .cart-trigger:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(2, 132, 199, 0.35);
  }
  .cart-trigger:active {
    transform: translateY(1px) scale(0.95);
  }
  .cart-trigger.cart-bump {
    animation: cartShake 0.5s ease-in-out;
  }
  .cart-count {
    background: #ef4444; color: white; font-size: 0.75rem; padding: 0.1rem 0.4rem; border-radius: 999px;
    transition: transform 0.2s ease;
  }
  .cart-count.count-pop {
    animation: countPop 0.45s cubic-bezier(0.175, 0.885, 0.32, 1.275);
  }

  @keyframes cartShake {
    0%, 100% { transform: rotate(0deg) scale(1); }
    20% { transform: rotate(-10deg) scale(1.08); }
    40% { transform: rotate(10deg) scale(1.08); }
    60% { transform: rotate(-5deg) scale(1.04); }
    80% { transform: rotate(5deg) scale(1.02); }
  }

  @keyframes countPop {
    0% { transform: scale(1); }
    50% { transform: scale(1.45); }
    100% { transform: scale(1); }
  }
</style>
