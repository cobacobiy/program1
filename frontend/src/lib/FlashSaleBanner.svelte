<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { apiFetch } from './api';
  import { formatRupiah } from './currency';
  import type { FlashSaleSessionDto, FlashSaleItemDto } from './types';

  interface Props {
    onAddToCart?: (item: FlashSaleItemDto) => void;
    onSelectProduct?: (productId: string) => void;
  }

  let { onAddToCart, onSelectProduct }: Props = $props();

  let session = $state<FlashSaleSessionDto | null>(null);
  let isLoading = $state(true);
  let remainingSeconds = $state(0);
  let timer: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    fetchActiveFlashSale();
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
  });

  async function fetchActiveFlashSale() {
    try {
      isLoading = true;
      const res = await apiFetch<FlashSaleSessionDto | null>('/api/v1/flash-sales/active');
      if (res && res.is_active && res.time_remaining_seconds > 0) {
        session = res;
        remainingSeconds = res.time_remaining_seconds;
        startCountdown();
      } else {
        session = null;
      }
    } catch {
      session = null;
    } finally {
      isLoading = false;
    }
  }

  function startCountdown() {
    if (timer) clearInterval(timer);
    timer = setInterval(() => {
      if (remainingSeconds > 0) {
        remainingSeconds -= 1;
      } else {
        if (timer) clearInterval(timer);
        fetchActiveFlashSale(); // Re-fetch on session end
      }
    }, 1000);
  }

  function formatTime(totalSec: number) {
    if (totalSec <= 0) return { hours: '00', minutes: '00', seconds: '00' };
    const hours = Math.floor(totalSec / 3600);
    const minutes = Math.floor((totalSec % 3600) / 60);
    const seconds = totalSec % 60;
    return {
      hours: String(hours).padStart(2, '0'),
      minutes: String(minutes).padStart(2, '0'),
      seconds: String(seconds).padStart(2, '0'),
    };
  }

  let formattedCountdown = $derived(formatTime(remainingSeconds));
</script>

{#if session && session.items && session.items.length > 0}
  <div class="flash-sale-wrapper">
    <div class="flash-sale-card">
      <!-- Header with Flame & Live Countdown -->
      <div class="banner-header">
        <div class="header-left">
          <div class="badge-pulse">
            <span class="lightning-icon">⚡</span>
            <span class="live-indicator">LIVE</span>
          </div>
          <div class="title-meta">
            <h2 class="sale-title">{session.title}</h2>
            {#if session.description}
              <p class="sale-desc">{session.description}</p>
            {/if}
          </div>
        </div>

        <div class="countdown-container">
          <span class="countdown-label">Berakhir dalam:</span>
          <div class="time-boxes">
            <div class="time-box">
              <span class="time-val">{formattedCountdown.hours}</span>
              <span class="time-unit">Jam</span>
            </div>
            <span class="time-sep">:</span>
            <div class="time-box">
              <span class="time-val">{formattedCountdown.minutes}</span>
              <span class="time-unit">Mnt</span>
            </div>
            <span class="time-sep">:</span>
            <div class="time-box highlight-sec">
              <span class="time-val">{formattedCountdown.seconds}</span>
              <span class="time-unit">Dtk</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Items Grid / Horizontal Scroll -->
      <div class="items-scroll-track">
        {#each session.items as item}
          {@const soldPercent = Math.min(100, Math.round((item.stock_sold / item.stock_allocated) * 100))}
          <div class="flash-item-card" class:sold-out={item.is_sold_out}>
            <!-- Image & Badges -->
            <div class="item-visual" onclick={() => onSelectProduct && onSelectProduct(item.product_id)}>
              {#if item.product_image_url}
                <img src={item.product_image_url} alt={item.product_name} class="item-img" />
              {:else}
                <div class="img-placeholder">🔥</div>
              {/if}
              <div class="discount-pill">-{item.discount_percentage}%</div>
              {#if item.is_sold_out}
                <div class="sold-out-overlay">
                  <span>HABIS</span>
                </div>
              {/if}
            </div>

            <!-- Details -->
            <div class="item-details">
              <h3
                class="item-name"
                title={item.product_name}
                onclick={() => onSelectProduct && onSelectProduct(item.product_id)}
              >
                {item.product_name}
              </h3>

              <div class="price-stack">
                <span class="flash-price">{formatRupiah(item.discount_price * 100)}</span>
                <span class="original-price">{formatRupiah(item.original_price * 100)}</span>
              </div>

              <!-- Sold Progress Bar -->
              <div class="stock-progress-section">
                <div class="progress-bar-bg">
                  <div class="progress-bar-fill" style="width: {soldPercent}%"></div>
                </div>
                <div class="stock-text">
                  {#if item.is_sold_out}
                    <span class="text-habis">Habis Terjual!</span>
                  {:else if soldPercent >= 80}
                    <span class="text-rush">🔥 Segera Habis! ({item.stock_remaining} sisa)</span>
                  {:else}
                    <span class="text-sold">Terjual {item.stock_sold}/{item.stock_allocated}</span>
                  {/if}
                </div>
              </div>

              <!-- Action Button -->
              <button
                type="button"
                class="buy-flash-btn"
                disabled={item.is_sold_out}
                onclick={() => onAddToCart && onAddToCart(item)}
              >
                {item.is_sold_out ? 'Habis' : '+ Keranjang'}
              </button>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .flash-sale-wrapper {
    margin: 1.5rem 0 2rem 0;
    width: 100%;
  }

  .flash-sale-card {
    background: linear-gradient(135deg, #1e1124 0%, #151828 50%, #0f172a 100%);
    border: 1px solid rgba(244, 63, 94, 0.35);
    border-radius: 18px;
    padding: 1.25rem 1.5rem;
    box-shadow: 0 10px 30px rgba(225, 29, 72, 0.15), 0 0 20px rgba(244, 63, 94, 0.1);
    position: relative;
    overflow: hidden;
  }

  .flash-sale-card::before {
    content: '';
    position: absolute;
    top: -50%;
    left: -50%;
    width: 200%;
    height: 200%;
    background: radial-gradient(circle at center, rgba(244, 63, 94, 0.08) 0%, transparent 60%);
    pointer-events: none;
  }

  .banner-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 1rem;
    margin-bottom: 1.25rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .badge-pulse {
    display: flex;
    align-items: center;
    gap: 6px;
    background: linear-gradient(90deg, #f43f5e, #fb7185);
    padding: 5px 12px;
    border-radius: 20px;
    font-weight: 800;
    font-size: 0.8rem;
    color: #ffffff;
    box-shadow: 0 0 16px rgba(244, 63, 94, 0.6);
    animation: glowPulse 2s infinite ease-in-out;
  }

  @keyframes glowPulse {
    0%, 100% {
      box-shadow: 0 0 12px rgba(244, 63, 94, 0.5);
    }
    50% {
      box-shadow: 0 0 22px rgba(244, 63, 94, 0.9);
    }
  }

  .lightning-icon {
    font-size: 1rem;
  }

  .live-indicator {
    letter-spacing: 0.08em;
  }

  .title-meta {
    display: flex;
    flex-direction: column;
  }

  .sale-title {
    font-size: 1.3rem;
    font-weight: 800;
    color: #ffffff;
    margin: 0;
    letter-spacing: -0.01em;
  }

  .sale-desc {
    font-size: 0.8rem;
    color: #94a3b8;
    margin: 2px 0 0 0;
  }

  .countdown-container {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.1);
    padding: 6px 14px;
    border-radius: 12px;
  }

  .countdown-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: #cbd5e1;
  }

  .time-boxes {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .time-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    padding: 3px 6px;
    min-width: 34px;
  }

  .time-val {
    font-size: 0.95rem;
    font-weight: 800;
    font-family: monospace;
    color: #ffffff;
    line-height: 1;
  }

  .highlight-sec .time-val {
    color: #fb7185;
  }

  .time-unit {
    font-size: 0.6rem;
    color: #64748b;
    text-transform: uppercase;
    margin-top: 2px;
  }

  .time-sep {
    font-size: 1rem;
    font-weight: bold;
    color: #f43f5e;
  }

  .items-scroll-track {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 14px;
    overflow-x: auto;
    padding-bottom: 4px;
  }

  @media (max-width: 640px) {
    .items-scroll-track {
      grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    }
  }

  .flash-item-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition: transform 0.2s ease, border-color 0.2s ease, box-shadow 0.2s ease;
  }

  .flash-item-card:hover {
    transform: translateY(-3px);
    border-color: rgba(244, 63, 94, 0.4);
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.4);
  }

  .flash-item-card.sold-out {
    opacity: 0.6;
    filter: grayscale(0.5);
  }

  .item-visual {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    background: #181f30;
    cursor: pointer;
    overflow: hidden;
  }

  .item-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.25s ease;
  }

  .flash-item-card:hover .item-img {
    transform: scale(1.05);
  }

  .img-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 2.5rem;
  }

  .discount-pill {
    position: absolute;
    top: 8px;
    left: 8px;
    background: #f43f5e;
    color: #ffffff;
    font-size: 0.75rem;
    font-weight: 800;
    padding: 2px 8px;
    border-radius: 6px;
    box-shadow: 0 2px 8px rgba(244, 63, 94, 0.6);
  }

  .sold-out-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #f87171;
    font-weight: 800;
    font-size: 0.9rem;
    letter-spacing: 0.05em;
  }

  .item-details {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    flex: 1;
  }

  .item-name {
    font-size: 0.85rem;
    font-weight: 600;
    color: #f1f5f9;
    margin: 0 0 6px 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
  }

  .item-name:hover {
    color: #fb7185;
  }

  .price-stack {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin-bottom: 8px;
  }

  .flash-price {
    font-size: 1rem;
    font-weight: 800;
    color: #f43f5e;
  }

  .original-price {
    font-size: 0.75rem;
    color: #64748b;
    text-decoration: line-through;
  }

  .stock-progress-section {
    margin-bottom: 10px;
  }

  .progress-bar-bg {
    width: 100%;
    height: 6px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #f43f5e, #fb923c);
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .stock-text {
    font-size: 0.7rem;
    margin-top: 3px;
  }

  .text-rush {
    color: #fb923c;
    font-weight: 600;
  }

  .text-habis {
    color: #ef4444;
    font-weight: 600;
  }

  .text-sold {
    color: #94a3b8;
  }

  .buy-flash-btn {
    margin-top: auto;
    width: 100%;
    padding: 7px;
    background: linear-gradient(90deg, #f43f5e, #e11d48);
    border: none;
    border-radius: 8px;
    color: #ffffff;
    font-size: 0.8rem;
    font-weight: 700;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.15s;
  }

  .buy-flash-btn:hover:not(:disabled) {
    opacity: 0.92;
    transform: scale(1.02);
  }

  .buy-flash-btn:disabled {
    background: rgba(255, 255, 255, 0.1);
    color: #64748b;
    cursor: not-allowed;
  }
</style>
