<script lang="ts">
  import { cart } from './cart.svelte';
  import { auth } from './auth.svelte';
  import { toast } from './toast.svelte';
  import { apiFetch } from './api';
  import { formatRupiah } from './currency';
  import { loadMidtransSnap } from './midtrans';
  import type {
    CreateOrderResponse,
    CouponValidationResult,
    Coupon,
    BuyerAddress,
    CourierRate,
  } from './types';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();
  let isSubmitting = $state(false);
  let notes = $state('');

  // Shipping & Address State
  let addresses = $state<BuyerAddress[]>([]);
  let selectedAddressId = $state('');
  let isLoadingAddresses = $state(false);
  let isAddingAddress = $state(false);

  // New Address Form
  let newRecipient = $state('');
  let newPhone = $state('');
  let newStreet = $state('');
  let newSubdistrict = $state('');
  let newCity = $state('Jakarta Selatan');
  let newProvince = $state('DKI Jakarta');
  let newPostalCode = $state('12190');
  let isSavingAddress = $state(false);

  // Shipping Cost State
  let availableCouriers = $state<CourierRate[]>([]);
  let selectedCourierKey = $state('');
  let isLoadingCouriers = $state(false);

  // Coupon / Promo state
  let promoInput = $state('');
  let isValidatingCoupon = $state(false);
  let appliedCoupon = $state<Coupon | null>(null);
  let discountAmount = $state(0);
  let promoError = $state<string | null>(null);

  // Weight calculation (grams)
  const totalWeightGrams = $derived(
    cart.items.reduce(
      (sum, item) => sum + (item.product.weight_grams || 500) * item.quantity,
      0,
    ),
  );

  const selectedAddress = $derived(
    addresses.find((a) => a.id === selectedAddressId) || addresses[0] || null,
  );

  const selectedCourier = $derived(
    availableCouriers.find(
      (c) => `${c.courier_code}-${c.service}` === selectedCourierKey,
    ) || availableCouriers[0] || null,
  );

  const shippingCostCents = $derived(selectedCourier ? selectedCourier.cost_cents : 0);

  const effectiveSubtotal = $derived(Math.max(0, cart.totalAmountCents - discountAmount));
  const finalGrandTotal = $derived(effectiveSubtotal + shippingCostCents);

  async function loadAddresses() {
    if (!auth.user) return;
    isLoadingAddresses = true;
    try {
      const data = await apiFetch<BuyerAddress[]>('/api/v1/buyer/addresses').catch(
        () => apiFetch<BuyerAddress[]>('/buyer/addresses'),
      );
      addresses = data || [];
      const defaultAddr = addresses.find((a) => a.is_default);
      if (defaultAddr) {
        selectedAddressId = defaultAddr.id;
      } else if (addresses.length > 0) {
        selectedAddressId = addresses[0].id;
      }
      if (addresses.length > 0) {
        await loadShippingRates();
      }
    } catch (e: any) {
      console.warn('Failed to load addresses', e);
    } finally {
      isLoadingAddresses = false;
    }
  }

  async function saveNewAddress(e: Event) {
    e.preventDefault();
    if (!newRecipient.trim() || !newPhone.trim() || !newStreet.trim()) {
      toast.error('Lengkapi nama penerima, nomor HP, dan alamat jalan');
      return;
    }
    isSavingAddress = true;
    try {
      const created = await apiFetch<BuyerAddress>('/api/v1/buyer/addresses', {
        method: 'POST',
        body: JSON.stringify({
          recipient_name: newRecipient.trim(),
          phone_number: newPhone.trim(),
          street_address: newStreet.trim(),
          subdistrict: newSubdistrict.trim() || 'Kecamatan',
          city: newCity.trim() || 'Jakarta Selatan',
          province: newProvince.trim() || 'DKI Jakarta',
          postal_code: newPostalCode.trim() || '12000',
          is_default: addresses.length === 0,
        }),
      });
      toast.success('Alamat pengiriman berhasil ditambahkan!');
      addresses = [...addresses, created];
      selectedAddressId = created.id;
      isAddingAddress = false;
      await loadShippingRates();
    } catch (err: any) {
      toast.error(err.message || 'Gagal menyimpan alamat.');
    } finally {
      isSavingAddress = false;
    }
  }

  async function loadShippingRates() {
    isLoadingCouriers = true;
    try {
      const rates = await apiFetch<CourierRate[]>('/api/v1/shipping/cost', {
        method: 'POST',
        body: JSON.stringify({
          destination_city_id: '152',
          weight_grams: Math.max(100, totalWeightGrams),
          courier: 'all',
        }),
      });
      availableCouriers = rates || [];
      if (availableCouriers.length > 0 && !selectedCourierKey) {
        selectedCourierKey = `${availableCouriers[0].courier_code}-${availableCouriers[0].service}`;
      }
    } catch (e: any) {
      console.warn('Failed to load shipping rates', e);
    } finally {
      isLoadingCouriers = false;
    }
  }

  async function handleApplyPromo() {
    if (!promoInput.trim()) return;
    isValidatingCoupon = true;
    promoError = null;
    try {
      const res = await apiFetch<CouponValidationResult>('/api/v1/coupons/validate', {
        method: 'POST',
        body: JSON.stringify({
          code: promoInput.trim().toUpperCase(),
          order_amount: cart.totalAmountCents,
        }),
      });

      if (res.is_valid) {
        appliedCoupon = res.coupon || null;
        discountAmount = res.discount_amount;
        toast.success(`Kupon ${res.coupon?.code || promoInput.toUpperCase()} berhasil diterapkan!`);
      } else {
        promoError = res.message || 'Kupon tidak valid atau telah kedaluwarsa.';
        appliedCoupon = null;
        discountAmount = 0;
        toast.error(promoError);
      }
    } catch (err: any) {
      const msg = err.message || 'Gagal memvalidasi kupon.';
      promoError = msg;
      appliedCoupon = null;
      discountAmount = 0;
      toast.error(msg);
    } finally {
      isValidatingCoupon = false;
    }
  }

  function handleRemovePromo() {
    appliedCoupon = null;
    discountAmount = 0;
    promoInput = '';
    promoError = null;
    toast.info('Kupon dibatalkan.');
  }

  async function handleCheckout(e: Event) {
    e.preventDefault();
    if (cart.items.length === 0) return;

    if (!auth.user) {
      toast.error('Silakan login terlebih dahulu untuk melakukan checkout.');
      auth.isModalOpen = true;
      return;
    }

    if (!selectedAddress && addresses.length === 0) {
      isAddingAddress = true;
      toast.error('Mohon isi alamat pengiriman terlebih dahulu');
      return;
    }

    isSubmitting = true;
    try {
      const config = await apiFetch<{ client_key: string }>('/buyer/payment-config').catch(
        () => ({ client_key: 'SB-Mid-client-dummy' }),
      );
      await loadMidtransSnap(config.client_key, false).catch(() => {});

      let checkoutNotes = notes.trim();
      if (appliedCoupon) {
        checkoutNotes = checkoutNotes
          ? `${checkoutNotes} [Kupon: ${appliedCoupon.code} (-${formatRupiah(discountAmount)})]`
          : `[Kupon: ${appliedCoupon.code} (-${formatRupiah(discountAmount)})]`;
      }

      const courierDesc = selectedCourier
        ? `${selectedCourier.courier_code.toUpperCase()} - ${selectedCourier.service}`
        : 'JNE - REG';

      const payload = {
        address_id: selectedAddress ? selectedAddress.id : undefined,
        items: cart.items.map((i) => ({
          product_id: i.product.id,
          quantity: i.quantity,
        })),
        courier: courierDesc,
        shipping_cost_cents: shippingCostCents,
        notes: checkoutNotes,
      };

      const res = await apiFetch<CreateOrderResponse>('/api/v1/orders', {
        method: 'POST',
        body: JSON.stringify(payload),
      }).catch(() =>
        apiFetch<CreateOrderResponse>('/v1/orders', {
          method: 'POST',
          body: JSON.stringify(payload),
        }),
      );

      if (res.snap_token && window.snap) {
        window.snap.pay(res.snap_token, {
          onSuccess: () => {
            toast.success('Pembayaran Berhasil! Pesanan sedang diproses.');
            cart.clear();
            handleRemovePromo();
            onClose();
          },
          onPending: () => {
            toast.info('Menunggu penyelesaian pembayaran.');
            cart.clear();
            handleRemovePromo();
            onClose();
          },
          onError: () => {
            toast.error('Pembayaran gagal atau dibatalkan.');
          },
          onClose: () => {
            toast.info('Jendela pembayaran ditutup.');
            cart.clear();
            handleRemovePromo();
            onClose();
          },
        });
      } else {
        toast.success(`Pesanan #${res.order_id.slice(0, 8)} berhasil dibuat!`);
        cart.clear();
        handleRemovePromo();
        onClose();
      }
    } catch (err: any) {
      toast.error(err.message || 'Gagal memproses pesanan.');
    } finally {
      isSubmitting = false;
    }
  }

  $effect(() => {
    if (isOpen && auth.user) {
      loadAddresses();
    }
  });
</script>

{#if isOpen}
  <div
    class="modal-overlay"
    onclick={onClose}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Escape' && onClose()}
  >
    <div
      class="modal-card"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div class="modal-header">
        <h3>💳 Checkout & Pengiriman</h3>
        <button class="close-btn" onclick={onClose}>&times;</button>
      </div>

      <!-- Shipping Address Section -->
      <div class="checkout-section">
        <div class="section-title-row">
          <span class="sec-title">📍 Alamat Pengiriman</span>
          {#if addresses.length > 0 && !isAddingAddress}
            <button
              type="button"
              class="btn-text"
              onclick={() => (isAddingAddress = !isAddingAddress)}
            >
              + Tambah Baru
            </button>
          {/if}
        </div>

        {#if isAddingAddress || addresses.length === 0}
          <form onsubmit={saveNewAddress} class="address-form">
            <div class="form-row">
              <input type="text" placeholder="Nama Penerima" bind:value={newRecipient} required />
              <input type="tel" placeholder="Nomor HP (WhatsApp)" bind:value={newPhone} required />
            </div>
            <textarea
              placeholder="Alamat Lengkap (Jalan, RT/RW, No. Rumah)"
              bind:value={newStreet}
              rows="2"
              required
            ></textarea>
            <div class="form-row">
              <input type="text" placeholder="Kecamatan" bind:value={newSubdistrict} />
              <input type="text" placeholder="Kota / Kabupaten" bind:value={newCity} />
            </div>
            <div class="form-row">
              <input type="text" placeholder="Provinsi" bind:value={newProvince} />
              <input type="text" placeholder="Kode Pos" bind:value={newPostalCode} />
            </div>
            <div class="address-actions">
              {#if addresses.length > 0}
                <button
                  type="button"
                  class="btn-cancel-addr"
                  onclick={() => (isAddingAddress = false)}
                >
                  Batal
                </button>
              {/if}
              <button type="submit" class="btn-save-addr" disabled={isSavingAddress}>
                {isSavingAddress ? 'Menyimpan...' : 'Simpan Alamat'}
              </button>
            </div>
          </form>
        {:else}
          <div class="address-selector">
            {#if addresses.length > 1}
              <select bind:value={selectedAddressId} onchange={() => loadShippingRates()}>
                {#each addresses as addr}
                  <option value={addr.id}>
                    {addr.recipient_name} ({addr.city}) {addr.is_default ? '★ Utama' : ''}
                  </option>
                {/each}
              </select>
            {/if}
            {#if selectedAddress}
              <div class="selected-addr-card">
                <div class="addr-header">
                  <strong>{selectedAddress.recipient_name}</strong>
                  <span class="phone">{selectedAddress.phone_number}</span>
                </div>
                <p class="addr-detail">
                  {selectedAddress.street_address}, {selectedAddress.subdistrict}, {selectedAddress.city}, {selectedAddress.province} {selectedAddress.postal_code}
                </p>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Courier / Ongkir Section -->
      <div class="checkout-section">
        <div class="section-title-row">
          <span class="sec-title">🚚 Opsi Kurir Ekspedisi</span>
          <span class="weight-badge">
            Berat: {totalWeightGrams >= 1000 ? `${(totalWeightGrams / 1000).toFixed(1)} kg` : `${totalWeightGrams} g`}
          </span>
        </div>

        {#if isLoadingCouriers}
          <p class="loading-text">Menghitung estimasi ongkos kirim...</p>
        {:else if availableCouriers.length === 0}
          <p class="empty-text">Pilih alamat untuk melihat kurir pengiriman yang tersedia.</p>
        {:else}
          <div class="couriers-list">
            {#each availableCouriers as courier}
              <label
                class="courier-card"
                class:selected={selectedCourierKey === `${courier.courier_code}-${courier.service}`}
              >
                <input
                  type="radio"
                  name="courier_option"
                  value="{courier.courier_code}-{courier.service}"
                  bind:group={selectedCourierKey}
                />
                <div class="courier-info">
                  <div class="courier-name">
                    <strong>{courier.courier_code.toUpperCase()}</strong>
                    <span class="service-name">{courier.service}</span>
                  </div>
                  <small class="etd">Estimasi: {courier.etd} hari kerja</small>
                </div>
                <div class="courier-cost">
                  {formatRupiah(courier.cost_cents)}
                </div>
              </label>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Order Summary -->
      <div class="order-summary">
        <div class="summary-row">
          <span>Total Item:</span>
          <strong>{cart.totalItems} barang</strong>
        </div>
        <div class="summary-row">
          <span>Subtotal Produk:</span>
          <span>{formatRupiah(cart.totalAmountCents)}</span>
        </div>
        {#if appliedCoupon}
          <div class="summary-row discount-row">
            <span>🏷️ Diskon ({appliedCoupon.code}):</span>
            <strong class="discount-val">-{formatRupiah(discountAmount)}</strong>
          </div>
        {/if}
        <div class="summary-row shipping-row">
          <span>
            🚚 Ongkir ({selectedCourier ? `${selectedCourier.courier_code.toUpperCase()} ${selectedCourier.service}` : 'Standar'}):
          </span>
          <strong>{formatRupiah(shippingCostCents)}</strong>
        </div>
        <div class="summary-row total-highlight">
          <span>Total Tagihan:</span>
          <strong class="price">{formatRupiah(finalGrandTotal)}</strong>
        </div>
      </div>

      <!-- Promo Code Box -->
      <div class="promo-box">
        {#if appliedCoupon}
          <div class="applied-badge">
            <span class="badge-text">
              🏷️ Kupon <strong>{appliedCoupon.code}</strong> aktif (-{formatRupiah(discountAmount)})
            </span>
            <button type="button" class="btn-remove-coupon" onclick={handleRemovePromo}>Batal</button>
          </div>
        {:else}
          <div class="promo-input-group">
            <input
              type="text"
              placeholder="Punya Kode Promo? (cth: HEMAT20)"
              bind:value={promoInput}
              onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), handleApplyPromo())}
            />
            <button
              type="button"
              class="btn-apply-promo"
              onclick={handleApplyPromo}
              disabled={isValidatingCoupon || !promoInput.trim()}
            >
              {isValidatingCoupon ? 'Cek...' : 'Terapkan'}
            </button>
          </div>
          {#if promoError}
            <p class="promo-error">{promoError}</p>
          {/if}
        {/if}
      </div>

      <form onsubmit={handleCheckout} class="checkout-form">
        <label>
          Catatan Tambahan (Opsional):
          <textarea bind:value={notes} rows="2" placeholder="Contoh: Packing kayu / warna hitam"></textarea>
        </label>

        <div class="modal-actions">
          <button type="button" class="btn-secondary" onclick={onClose} disabled={isSubmitting}>
            Batal
          </button>
          <button
            type="submit"
            class="btn-primary"
            disabled={isSubmitting || cart.items.length === 0 || (!selectedAddress && addresses.length === 0)}
          >
            {isSubmitting ? 'Menyiapkan Transaksi...' : `Bayar Sekarang (${formatRupiah(finalGrandTotal)}) 🚀`}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.75);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
    backdrop-filter: blur(4px);
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 14px;
    width: 92%; max-width: 500px; max-height: 92vh; overflow-y: auto; padding: 1.5rem; color: #f8fafc;
    box-shadow: 0 15px 35px rgba(0,0,0,0.6);
  }
  .modal-header {
    display: flex; justify-content: space-between; align-items: center;
    border-bottom: 1px solid #334155; padding-bottom: 0.75rem; margin-bottom: 1rem;
  }
  .modal-header h3 { margin: 0; font-size: 1.25rem; font-weight: 700; color: #f8fafc; }
  .close-btn { background: none; border: none; font-size: 1.5rem; color: #94a3b8; cursor: pointer; }

  /* Sections */
  .checkout-section {
    background: #1e293b; border: 1px solid #334155; border-radius: 10px;
    padding: 0.9rem; margin-bottom: 1rem;
  }
  .section-title-row {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 0.6rem;
  }
  .sec-title { font-size: 0.9rem; font-weight: 600; color: #38bdf8; }
  .btn-text {
    background: none; border: none; color: #0284c7; font-size: 0.8rem;
    cursor: pointer; font-weight: 600; padding: 0;
  }
  .btn-text:hover { color: #38bdf8; text-decoration: underline; }
  .weight-badge {
    font-size: 0.75rem; background: #334155; color: #cbd5e1;
    padding: 0.2rem 0.5rem; border-radius: 6px;
  }

  /* Address styles */
  .selected-addr-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 8px; padding: 0.75rem;
  }
  .addr-header {
    display: flex; justify-content: space-between; font-size: 0.88rem; margin-bottom: 0.25rem;
  }
  .phone { color: #94a3b8; font-size: 0.8rem; }
  .addr-detail { margin: 0; font-size: 0.8rem; color: #cbd5e1; line-height: 1.35; }
  .address-selector select {
    width: 100%; background: #0f172a; border: 1px solid #334155; border-radius: 6px;
    color: #fff; padding: 0.45rem; font-size: 0.82rem; margin-bottom: 0.5rem;
  }

  .address-form { display: flex; flex-direction: column; gap: 0.5rem; }
  .form-row { display: flex; gap: 0.5rem; }
  .address-form input, .address-form textarea {
    flex: 1; background: #0f172a; border: 1px solid #334155; border-radius: 6px;
    padding: 0.45rem 0.65rem; color: #fff; font-size: 0.82rem; font-family: inherit;
  }
  .address-actions { display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 0.25rem; }
  .btn-save-addr {
    background: #0284c7; color: #fff; border: none; padding: 0.4rem 0.8rem;
    border-radius: 6px; font-size: 0.8rem; font-weight: 600; cursor: pointer;
  }
  .btn-cancel-addr {
    background: #334155; color: #fff; border: none; padding: 0.4rem 0.8rem;
    border-radius: 6px; font-size: 0.8rem; cursor: pointer;
  }

  /* Couriers list */
  .couriers-list { display: flex; flex-direction: column; gap: 0.45rem; max-height: 160px; overflow-y: auto; }
  .courier-card {
    display: flex; align-items: center; gap: 0.6rem; background: #0f172a;
    border: 1px solid #334155; border-radius: 8px; padding: 0.55rem 0.75rem;
    cursor: pointer; transition: all 0.15s ease;
  }
  .courier-card:hover { border-color: #0284c7; }
  .courier-card.selected { border-color: #38bdf8; background: #0b1f3a; }
  .courier-info { flex: 1; display: flex; flex-direction: column; }
  .courier-name { display: flex; gap: 0.35rem; align-items: center; font-size: 0.85rem; }
  .service-name { color: #94a3b8; font-size: 0.8rem; }
  .etd { color: #64748b; font-size: 0.72rem; }
  .courier-cost { font-weight: 700; font-size: 0.88rem; color: #38bdf8; }
  .loading-text, .empty-text { font-size: 0.8rem; color: #94a3b8; margin: 0.25rem 0; }

  /* Order Summary */
  .order-summary {
    background: #1e293b; padding: 0.9rem; border-radius: 10px; border: 1px solid #334155;
    margin-bottom: 1rem; display: flex; flex-direction: column; gap: 0.4rem;
  }
  .summary-row { display: flex; justify-content: space-between; font-size: 0.9rem; }
  .discount-row { color: #10b981; }
  .discount-val { color: #10b981; font-weight: bold; }
  .shipping-row { color: #cbd5e1; }
  .total-highlight { border-top: 1px solid #334155; padding-top: 0.5rem; font-size: 1.05rem; }
  .price { color: #38bdf8; }

  /* Promo Box */
  .promo-box { margin-bottom: 1rem; }
  .promo-input-group { display: flex; gap: 0.5rem; }
  .promo-input-group input {
    flex: 1; background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.5rem 0.75rem; color: #fff; font-size: 0.85rem; text-transform: uppercase;
  }
  .btn-apply-promo {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 0.9rem;
    border-radius: 6px; font-weight: bold; cursor: pointer; font-size: 0.82rem;
  }
  .applied-badge {
    background: #064e3b; border: 1px solid #059669; padding: 0.55rem 0.75rem;
    border-radius: 6px; display: flex; justify-content: space-between; align-items: center;
  }
  .badge-text { font-size: 0.82rem; color: #a7f3d0; }
  .btn-remove-coupon {
    background: #7f1d1d; color: #fecaca; border: none; padding: 0.2rem 0.5rem;
    border-radius: 4px; font-size: 0.75rem; cursor: pointer;
  }
  .promo-error { font-size: 0.78rem; color: #f87171; margin-top: 0.25rem; }

  .checkout-form { display: flex; flex-direction: column; gap: 0.85rem; }
  label { font-size: 0.82rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.3rem; }
  textarea {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.5rem; color: #fff; font-family: inherit; resize: vertical;
  }
  .modal-actions { display: flex; justify-content: flex-end; gap: 0.6rem; margin-top: 0.25rem; }
  .btn-secondary {
    background: #334155; color: #fff; border: none; padding: 0.6rem 1rem;
    border-radius: 6px; cursor: pointer; font-weight: 500; font-size: 0.88rem;
  }
  .btn-primary {
    background: #0284c7; color: #fff; border: none; padding: 0.6rem 1.2rem;
    border-radius: 6px; cursor: pointer; font-weight: bold; font-size: 0.88rem;
  }
  .btn-primary:hover:not(:disabled) { background: #0369a1; }
  .btn-primary:disabled { background: #475569; cursor: not-allowed; }
</style>
