/* ==========================================================================
   STOREFRONT LOGIC ENGINE (SHOPEE-STYLE RESPONSIVE WITH BUYER LOGIN)
   Target File: /home/cacyos/Downloads/github/program1/crates/web/static/store.js
   ========================================================================== */

// --- MODERN IN-APP TOAST SYSTEM (ELIMINATES BROWSER POPUPS & IP EXPOSURE) ---
function showToast(message, type = "info", duration = 4000) {
  let container = document.getElementById("store-toast-container");
  if (!container) {
    container = document.createElement("div");
    container.id = "store-toast-container";
    container.className = "store-toast-container";
    document.body.appendChild(container);
  }

  const toast = document.createElement("div");
  toast.className = `store-toast ${type}`;

  let icon = "ℹ️";
  if (type === "success") icon = "✅";
  if (type === "warning") icon = "⚠️";
  if (type === "error") icon = "❌";

  toast.innerHTML = `<span>${icon}</span><div style="flex:1">${message}</div>`;
  container.appendChild(toast);

  setTimeout(() => {
    toast.style.opacity = "0";
    toast.style.transform = "translateX(100%)";
    setTimeout(() => toast.remove(), 300);
  }, duration);
}

// Override default window.alert so browser dialogs displaying host IP addresses never appear
window.alert = function(msg) {
  let t = "info";
  const str = String(msg);
  if (str.includes("🎉") || str.includes("✅") || str.includes("berhasil") || str.includes("Berhasil")) {
    t = "success";
  } else if (str.includes("Gagal") || str.includes("Error") || str.includes("error") || str.includes("❌")) {
    t = "error";
  } else if (str.includes("Harap") || str.includes("Silakan") || str.includes("🔒") || str.includes("⚠️") || str.includes("kosong")) {
    t = "warning";
  }
  showToast(str, t);
};

let cart = [];
let catalog = [];
let activeCategory = "ALL";
let searchQuery = "";
let buyerToken = localStorage.getItem("program1_buyer_token") || null;
let activeBuyer = null;
let buyerAddresses = [];
let selectedAddressId = null;
let otpTimerInterval = null;
let otpSecondsRemaining = 300;
let currentOtpPhone = "";
let googleClientId = null;
let gisRenderAttempts = 0;
let storeInfo = null;
let storeWhatsAppNumber = "085810007735";
let inAppChatMessages = [];

// --- THEME ENGINE (DARK / LIGHT) ---
function initStoreTheme() {
  const savedTheme = localStorage.getItem("shopee_store_theme") || "dark";
  applyStoreTheme(savedTheme);
}

function applyStoreTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("shopee_store_theme", theme);
  const icon = document.getElementById("store-theme-icon");
  if (icon) {
    icon.innerText = theme === "light" ? "🌙" : "☀️";
  }
}

function toggleStoreTheme() {
  const currentTheme = document.documentElement.getAttribute("data-theme") || "dark";
  const newTheme = currentTheme === "light" ? "dark" : "light";
  applyStoreTheme(newTheme);
}

window.toggleStoreTheme = toggleStoreTheme;
initStoreTheme();

// Initialize Storefront
async function initStore() {
  try {
    initStoreTheme();
    // 1. Check Storefront Buyer Session
    checkBuyerSession();
    await fetchBuyerAuthConfig();

    // 2. Fetch Store Information
    const infoRes = await fetch('/api/v1/store/info');
    if (infoRes.ok) {
      storeInfo = await infoRes.json();
      storeWhatsAppNumber = storeInfo.whatsapp_number || "085810007735";
      const nameEl = document.getElementById('header-store-name');
      if (nameEl) nameEl.innerText = storeInfo.store_name || "AURA Storefront";

      const titleEl = document.getElementById('page-title');
      if (titleEl) titleEl.innerText = `${storeInfo.store_name || "AURA Storefront"} — Shopee Official Store`;

      updateFloatingChatWidget();
    }

    // 3. Fetch Catalog Products
    const res = await fetch('/api/v1/catalog');
    if (res.ok) {
      catalog = await res.json();
      renderCategoryPills();
      renderCatalog();
    }

    // 4. Start Flash Sale Countdown Timer
    startFlashSaleTimer();

    // 5. Setup Event Listeners
    setupSearchListener();
  } catch (e) {
    console.error("Failed to initialize storefront data:", e);
  }
}

// --- BUYER AUTHENTICATION, GOOGLE OAUTH, OTP & ADDRESS ENGINE ---

async function buyerAuthFetch(url, options = {}) {
  if (!buyerToken) {
    throw new Error("Authentication required. Please log in.");
  }
  options.headers = options.headers || {};
  options.headers["Authorization"] = `Bearer ${buyerToken}`;
  const res = await fetch(url, options);
  if (res.status === 401 || res.status === 403) {
    console.warn("Buyer token invalid or expired. Logging out...");
    logoutBuyer();
  }
  return res;
}

async function checkBuyerSession() {
  buyerToken = localStorage.getItem("program1_buyer_token") || null;
  const saved = localStorage.getItem("program1_buyer_user");
  if (buyerToken && saved) {
    try {
      activeBuyer = JSON.parse(saved);
      renderBuyerHeaderState();
      // Sync latest buyer profile & addresses from server
      await syncBuyerProfile();
    } catch (e) {
      logoutBuyer();
    }
  } else {
    renderBuyerHeaderState();
  }
}

async function syncBuyerProfile() {
  if (!buyerToken) return;
  try {
    const res = await buyerAuthFetch("/api/v1/buyer/profile");
    if (res.ok) {
      activeBuyer = await res.json();
      localStorage.setItem("program1_buyer_user", JSON.stringify(activeBuyer));
      renderBuyerHeaderState();
      await fetchBuyerAddresses();
    }
  } catch (e) {
    console.error("Error syncing buyer profile:", e);
  }
}

function renderBuyerHeaderState() {
  const btnLogin = document.getElementById("btn-buyer-login");
  const badgeProfile = document.getElementById("buyer-profile-badge");
  const btnOrders = document.getElementById("btn-buyer-orders");
  const avatarEl = document.getElementById("buyer-avatar");
  const nameLabelEl = document.getElementById("buyer-name-label");

  if (activeBuyer && buyerToken) {
    if (btnLogin) btnLogin.style.display = "none";
    if (badgeProfile) badgeProfile.style.display = "flex";
    if (btnOrders) btnOrders.style.display = "inline-flex";
    if (avatarEl) avatarEl.innerText = (activeBuyer.full_name || "P").charAt(0).toUpperCase();
    if (nameLabelEl) nameLabelEl.innerText = activeBuyer.full_name || "Pembeli";
  } else {
    if (btnLogin) btnLogin.style.display = "block";
    if (badgeProfile) badgeProfile.style.display = "none";
    if (btnOrders) btnOrders.style.display = "none";
  }
  updateFloatingChatWidget();
}

function openBuyerLoginModal() {
  const modal = document.getElementById("buyer-login-modal");
  const loggedView = document.getElementById("buyer-logged-in-view");
  const unauthView = document.getElementById("buyer-unauthenticated-view");

  if (activeBuyer && buyerToken) {
    if (loggedView) loggedView.style.display = "block";
    if (unauthView) unauthView.style.display = "none";
    const avatarEl = document.getElementById("modal-buyer-avatar");
    const nameEl = document.getElementById("modal-buyer-name");
    const emailEl = document.getElementById("modal-buyer-email");
    if (avatarEl) avatarEl.innerText = (activeBuyer.full_name || "P").charAt(0).toUpperCase();
    if (nameEl) nameEl.innerText = activeBuyer.full_name || "Pembeli";
    if (emailEl) emailEl.innerText = activeBuyer.email || "-";

    const phoneBadge = document.getElementById("buyer-phone-status-badge");
    if (phoneBadge) {
      if (activeBuyer.phone_verified && activeBuyer.phone_number) {
        phoneBadge.innerHTML = `<span style="font-size:0.8rem; color:var(--emerald); font-weight:600">🟢 No. HP Terverifikasi: ${activeBuyer.phone_number}</span>`;
      } else {
        phoneBadge.innerHTML = `
          <div style="display:flex; align-items:center; gap:0.5rem">
            <span style="font-size:0.8rem; color:var(--rose); font-weight:600">🔴 Belum Verifikasi No. HP</span>
            <button class="btn-sm-action" onclick="openOtpModal()">Verifikasi Sekarang</button>
          </div>
        `;
      }
    }

    fetchBuyerAddresses();
  } else {
    if (loggedView) loggedView.style.display = "none";
    if (unauthView) unauthView.style.display = "block";
    renderGoogleSignInButton();
  }

  if (modal) modal.style.display = "flex";
}

function closeBuyerLoginModal() {
  const modal = document.getElementById("buyer-login-modal");
  if (modal) modal.style.display = "none";
  const contextNotice = document.getElementById("buyer-login-context-notice");
  if (contextNotice) contextNotice.style.display = "none";
}

async function fetchBuyerAuthConfig() {
  try {
    const res = await fetch("/api/v1/buyer/auth/config");
    if (res.ok) {
      const data = await res.json();
      googleClientId = data.google_client_id || null;
      renderGoogleSignInButton();
    }
  } catch (e) {
    console.warn("Gagal memuat konfigurasi Google Client ID:", e);
  }
}

function switchBuyerAuthTab(tab) {
  const loginTabBtn = document.getElementById("tab-btn-login");
  const regTabBtn = document.getElementById("tab-btn-register");
  const loginForm = document.getElementById("buyer-login-form");
  const regForm = document.getElementById("buyer-register-form");
  const loginErr = document.getElementById("buyer-login-error");
  const regErr = document.getElementById("buyer-reg-error");

  if (loginErr) loginErr.style.display = "none";
  if (regErr) regErr.style.display = "none";

  if (tab === "register") {
    if (loginTabBtn) loginTabBtn.classList.remove("active");
    if (regTabBtn) regTabBtn.classList.add("active");
    if (loginForm) loginForm.classList.remove("active");
    if (regForm) regForm.classList.add("active");
  } else {
    if (regTabBtn) regTabBtn.classList.remove("active");
    if (loginTabBtn) loginTabBtn.classList.add("active");
    if (regForm) regForm.classList.remove("active");
    if (loginForm) loginForm.classList.add("active");
  }
}

async function handleBuyerLogin(e) {
  if (e && e.preventDefault) e.preventDefault();
  const emailInput = document.getElementById("buyer-login-email");
  const passwordInput = document.getElementById("buyer-login-password");
  const submitBtn = document.getElementById("btn-submit-buyer-login");
  const errorBox = document.getElementById("buyer-login-error");

  if (errorBox) errorBox.style.display = "none";

  const email = emailInput ? emailInput.value.trim() : "";
  const password = passwordInput ? passwordInput.value : "";

  if (!email || !password) {
    if (errorBox) {
      errorBox.innerText = "Email dan kata sandi harus diisi";
      errorBox.style.display = "block";
    }
    return;
  }

  if (submitBtn) {
    submitBtn.disabled = true;
    submitBtn.innerText = "Memproses Masuk...";
  }

  try {
    const res = await fetch("/api/v1/buyer/auth/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    });

    const data = await res.json();
    if (!res.ok) {
      const msg = (data && (data.message || data.error)) || "Gagal masuk. Periksa email atau kata sandi Anda.";
      if (errorBox) {
        errorBox.innerText = msg;
        errorBox.style.display = "block";
      }
      return;
    }

    // Success
    buyerToken = data.access_token || data.token;
    activeBuyer = data.buyer;
    localStorage.setItem("program1_buyer_token", buyerToken);
    localStorage.setItem("program1_buyer_user", JSON.stringify(activeBuyer));
    renderBuyerHeaderState();
    closeBuyerLoginModal();

    alert(`🎉 Selamat datang kembali, ${activeBuyer.full_name}!`);

    if (data.requires_phone_verification || !activeBuyer.phone_verified) {
      openOtpModal();
    } else {
      await fetchBuyerAddresses();
      updateCartUI();
    }
  } catch (err) {
    if (errorBox) {
      errorBox.innerText = "Terjadi gangguan koneksi ke server: " + (err.message || err);
      errorBox.style.display = "block";
    }
  } finally {
    if (submitBtn) {
      submitBtn.disabled = false;
      submitBtn.innerText = "Masuk ke Akun Toko";
    }
  }
}

async function handleBuyerRegister(e) {
  if (e && e.preventDefault) e.preventDefault();
  const nameInput = document.getElementById("buyer-reg-name");
  const emailInput = document.getElementById("buyer-reg-email");
  const passwordInput = document.getElementById("buyer-reg-password");
  const confirmPasswordInput = document.getElementById("buyer-reg-confirm-password");
  const submitBtn = document.getElementById("btn-submit-buyer-reg");
  const errorBox = document.getElementById("buyer-reg-error");

  if (errorBox) errorBox.style.display = "none";

  const full_name = nameInput ? nameInput.value.trim() : "";
  const email = emailInput ? emailInput.value.trim() : "";
  const password = passwordInput ? passwordInput.value : "";
  const confirmPassword = confirmPasswordInput ? confirmPasswordInput.value : "";

  if (!full_name || full_name.length < 2) {
    if (errorBox) {
      errorBox.innerText = "Nama lengkap minimal 2 karakter";
      errorBox.style.display = "block";
    }
    return;
  }

  if (!email || !email.includes("@")) {
    if (errorBox) {
      errorBox.innerText = "Format email tidak valid";
      errorBox.style.display = "block";
    }
    return;
  }

  if (!password || password.length < 8) {
    if (errorBox) {
      errorBox.innerText = "Kata sandi minimal 8 karakter";
      errorBox.style.display = "block";
    }
    return;
  }

  if (password !== confirmPassword) {
    if (errorBox) {
      errorBox.innerText = "Konfirmasi kata sandi tidak cocok dengan kata sandi";
      errorBox.style.display = "block";
    }
    return;
  }

  if (submitBtn) {
    submitBtn.disabled = true;
    submitBtn.innerText = "Mendaftarkan Member...";
  }

  try {
    const res = await fetch("/api/v1/buyer/auth/register", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ full_name, email, password }),
    });

    const data = await res.json();
    if (!res.ok) {
      const msg = (data && (data.message || data.error)) || "Pendaftaran gagal. Pastikan email belum terdaftar.";
      if (errorBox) {
        errorBox.innerText = msg;
        errorBox.style.display = "block";
      }
      return;
    }

    // Success
    buyerToken = data.access_token || data.token;
    activeBuyer = data.buyer;
    localStorage.setItem("program1_buyer_token", buyerToken);
    localStorage.setItem("program1_buyer_user", JSON.stringify(activeBuyer));
    renderBuyerHeaderState();
    closeBuyerLoginModal();

    alert(`🎉 Pendaftaran berhasil! Selamat bergabung, ${activeBuyer.full_name}.`);

    if (data.requires_phone_verification || !activeBuyer.phone_verified) {
      openOtpModal();
    } else {
      await fetchBuyerAddresses();
      updateCartUI();
    }
  } catch (err) {
    if (errorBox) {
      errorBox.innerText = "Terjadi gangguan koneksi ke server: " + (err.message || err);
      errorBox.style.display = "block";
    }
  } finally {
    if (submitBtn) {
      submitBtn.disabled = false;
      submitBtn.innerText = "Daftar Member Sekarang";
    }
  }
}

async function devQuickBuyerLogin() {
  const timestamp = Date.now().toString().slice(-4);
  const demoEmail = `demo.buyer${timestamp}@store.local`;
  const demoPass = "Demo12345!";

  try {
    const res = await fetch("/api/v1/buyer/auth/register", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        full_name: `Buyer Demo #${timestamp}`,
        email: demoEmail,
        password: demoPass,
      }),
    });

    if (res.ok) {
      const data = await res.json();
      buyerToken = data.access_token || data.token;
      activeBuyer = data.buyer;
      localStorage.setItem("program1_buyer_token", buyerToken);
      localStorage.setItem("program1_buyer_user", JSON.stringify(activeBuyer));
      renderBuyerHeaderState();
      closeBuyerLoginModal();
      alert(`⚡ Berhasil login instan demo sebagai: ${activeBuyer.full_name}`);
      openOtpModal();
    } else {
      const err = await res.json();
      alert(`Gagal login demo: ${err.message || err.error}`);
    }
  } catch (e) {
    alert(`Error: ${e.message}`);
  }
}

function renderGoogleSignInButton() {
  const btnContainer = document.getElementById("google-signin-btn");
  const unconfiguredMsg = document.getElementById("google-signin-unconfigured");
  if (!btnContainer) return;

  if (!googleClientId || googleClientId.trim() === "" || googleClientId.includes("your-google-client-id")) {
    if (unconfiguredMsg) {
      unconfiguredMsg.innerText = "Google Sign-In belum dikonfigurasi resmi di server. Silakan daftar / masuk menggunakan form email di atas.";
      unconfiguredMsg.style.display = "block";
    }
    return;
  }

  if (unconfiguredMsg) unconfiguredMsg.style.display = "none";

  if (typeof window.google === "undefined" || !window.google.accounts || !window.google.accounts.id) {
    gisRenderAttempts++;
    if (gisRenderAttempts < 15) {
      setTimeout(renderGoogleSignInButton, 300);
    } else {
      if (unconfiguredMsg) {
        unconfiguredMsg.innerText = "SDK Google Sign-In tidak dapat dimuat. Pastikan jaringan internet aktif.";
        unconfiguredMsg.style.display = "block";
      }
    }
    return;
  }

  gisRenderAttempts = 0;
  btnContainer.innerHTML = "";

  try {
    window.google.accounts.id.initialize({
      client_id: googleClientId,
      callback: handleGoogleCredentialResponse,
      auto_select: false,
      cancel_on_tap_outside: true
    });

    window.google.accounts.id.renderButton(btnContainer, {
      theme: "outline",
      size: "large",
      shape: "rectangular",
      text: "continue_with",
      width: 320,
      locale: "id"
    });
  } catch (err) {
    console.error("Gagal merender tombol Google Sign-In:", err);
  }
}

async function handleGoogleCredentialResponse(response) {
  if (!response || !response.credential) {
    console.error("Google Sign-In response missing credential:", response);
    return;
  }
  await verifyGoogleIdToken(response.credential);
}

async function verifyGoogleIdToken(idToken) {
  try {
    const res = await fetch("/api/v1/buyer/auth/google", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id_token: idToken })
    });

    if (res.ok) {
      const data = await res.json();
      buyerToken = data.access_token || data.token;
      activeBuyer = data.buyer;
      localStorage.setItem("program1_buyer_token", buyerToken);
      localStorage.setItem("program1_buyer_user", JSON.stringify(activeBuyer));
      renderBuyerHeaderState();
      closeBuyerLoginModal();

      alert(`🎉 Selamat datang, ${activeBuyer.full_name}! Login Google berhasil.`);

      if (data.requires_phone_verification || !activeBuyer.phone_verified) {
        openOtpModal();
      } else {
        await fetchBuyerAddresses();
        updateCartUI();
      }
    } else {
      const err = await res.json();
      alert(`Gagal login dengan Google: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (e) {
    console.error("Google Auth error:", e);
    alert(`Error: ${e.message}`);
  }
}

function logoutBuyer() {
  buyerToken = null;
  activeBuyer = null;
  buyerAddresses = [];
  selectedAddressId = null;
  localStorage.removeItem("program1_buyer_token");
  localStorage.removeItem("program1_buyer_user");
  renderBuyerHeaderState();
  closeBuyerLoginModal();
  updateCartUI();
}

// --- OTP PHONE VERIFICATION ENGINE ---
function openOtpModal() {
  closeBuyerLoginModal();
  const modal = document.getElementById("buyer-otp-modal");
  const stepPhone = document.getElementById("otp-step-phone");
  const stepVerify = document.getElementById("otp-step-verify");
  if (stepPhone) stepPhone.style.display = "block";
  if (stepVerify) stepVerify.style.display = "none";
  if (activeBuyer && activeBuyer.phone_number) {
    const phoneInput = document.getElementById("otp-input-phone");
    if (phoneInput) phoneInput.value = activeBuyer.phone_number;
  }
  if (modal) modal.style.display = "flex";
}

function closeBuyerOtpModal() {
  const modal = document.getElementById("buyer-otp-modal");
  if (modal) modal.style.display = "none";
  if (otpTimerInterval) clearInterval(otpTimerInterval);
}

async function handleRequestOtp() {
  const phoneInput = document.getElementById("otp-input-phone");
  const phone = phoneInput ? phoneInput.value.trim() : "";
  if (!phone || phone.length < 8) {
    alert("Harap masukkan nomor handphone / WhatsApp yang valid (minimal 8 digit).");
    return;
  }

  const btn = document.getElementById("btn-request-otp");
  if (btn) {
    btn.disabled = true;
    btn.innerText = "Mengirim OTP...";
  }

  try {
    const res = await buyerAuthFetch("/api/v1/buyer/otp/request", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ phone_number: phone })
    });

    if (res.ok) {
      const data = await res.json();
      currentOtpPhone = phone;
      const sentLabel = document.getElementById("otp-sent-phone-label");
      if (sentLabel) sentLabel.innerText = phone;
      document.getElementById("otp-step-phone").style.display = "none";
      document.getElementById("otp-step-verify").style.display = "block";
      startOtpCountdown(300);

      // Setup WA Admin link
      const waLink = document.getElementById("otp-wa-admin-link");
      if (waLink) {
        const storeWa = window.STORE_WHATSAPP || storeWhatsAppNumber || "085810007735";
        let cleanWa = storeWa.replace(/[^0-9]/g, "");
        if (cleanWa.startsWith("08")) {
          cleanWa = "628" + cleanWa.substring(2);
        }
        if (!cleanWa) cleanWa = "6285810007735";
        const text = encodeURIComponent(`Halo Admin, saya ingin konfirmasi verifikasi nomor toko saya: ${phone}`);
        waLink.href = `https://wa.me/${cleanWa}?text=${text}`;
      }

      // Handle simulated/dev OTP
      const codeInput = document.getElementById("otp-input-code");
      const devNotice = document.getElementById("otp-dev-notice");
      if (data && data.dev_otp) {
        if (codeInput) codeInput.value = data.dev_otp;
        if (devNotice) {
          devNotice.style.display = "block";
          devNotice.innerHTML = `💡 <strong>Mode Demo / Simulasi:</strong> Gateway WhatsApp belum diset token Fonnte. Kode OTP Anda otomatis terisi: <span style="font-family:'JetBrains Mono'; font-size:1.15rem; color:#facc15; font-weight:bold; letter-spacing:2px">${data.dev_otp}</span>.<br>Klik tombol <strong>Verifikasi Kode OTP</strong> di bawah untuk langsung lanjut!`;
        }
      } else {
        if (codeInput) codeInput.value = "";
        if (devNotice) devNotice.style.display = "none";
        alert(`📲 Kode OTP telah dikirimkan ke WhatsApp/SMS nomor ${phone}.`);
      }
    } else {
      const err = await res.json();
      alert(`Gagal mengirim OTP: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (e) {
    console.error("Request OTP error:", e);
    alert(`Error: ${e.message}`);
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.innerText = "📲 Kirim Kode OTP";
    }
  }
}

function startOtpCountdown(durationSeconds) {
  if (otpTimerInterval) clearInterval(otpTimerInterval);
  otpSecondsRemaining = durationSeconds;
  const timerLabel = document.getElementById("otp-timer-label");

  function update() {
    const m = Math.floor(otpSecondsRemaining / 60);
    const s = otpSecondsRemaining % 60;
    if (timerLabel) {
      timerLabel.innerText = `${m < 10 ? '0' : ''}${m}:${s < 10 ? '0' : ''}${s}`;
    }
    if (otpSecondsRemaining <= 0) {
      clearInterval(otpTimerInterval);
      if (timerLabel) timerLabel.innerText = "KEDALUWARSA";
    } else {
      otpSecondsRemaining--;
    }
  }
  update();
  otpTimerInterval = setInterval(update, 1000);
}

async function handleVerifyOtp() {
  const codeInput = document.getElementById("otp-input-code");
  const code = codeInput ? codeInput.value.trim() : "";
  if (!code || code.length !== 6) {
    alert("Harap masukkan 6 digit kode OTP.");
    return;
  }

  const btn = document.getElementById("btn-submit-verify-otp");
  if (btn) {
    btn.disabled = true;
    btn.innerText = "Memverifikasi...";
  }

  try {
    const res = await buyerAuthFetch("/api/v1/buyer/otp/verify", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ phone_number: currentOtpPhone, code: code })
    });

    if (res.ok) {
      const result = await res.json();
      activeBuyer = (result && result.id) ? result : (result.buyer || result);
      localStorage.setItem("program1_buyer_user", JSON.stringify(activeBuyer));
      if (otpTimerInterval) clearInterval(otpTimerInterval);
      closeBuyerOtpModal();
      renderBuyerHeaderState();
      alert("🎉 Nomor HP Anda berhasil diverifikasi! Alamat pengiriman sekarang dapat digunakan.");
      await fetchBuyerAddresses();
      updateCartUI();
    } else {
      const err = await res.json();
      alert(`Verifikasi OTP Gagal: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (e) {
    console.error("Verify OTP error:", e);
    alert(`Error: ${e.message}`);
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.innerText = "✅ Verifikasi Kode OTP";
    }
  }
}

function handleResendOtp() {
  if (otpTimerInterval) clearInterval(otpTimerInterval);
  document.getElementById("otp-step-phone").style.display = "block";
  document.getElementById("otp-step-verify").style.display = "none";
  const codeInput = document.getElementById("otp-input-code");
  if (codeInput) codeInput.value = "";
  const devNotice = document.getElementById("otp-dev-notice");
  if (devNotice) devNotice.style.display = "none";
}

// --- BUYER ADDRESS BOOK ENGINE ---
async function fetchBuyerAddresses() {
  if (!buyerToken) return;
  try {
    const res = await buyerAuthFetch("/api/v1/buyer/addresses");
    if (res.ok) {
      buyerAddresses = await res.json();
      const defaultAddr = buyerAddresses.find(a => a.is_default) || buyerAddresses[0];
      if (defaultAddr) {
        selectedAddressId = defaultAddr.id;
      }
      renderBuyerAddressesList();
      renderConfirmedAddressInCheckout();
    }
  } catch (e) {
    console.error("Fetch buyer addresses error:", e);
  }
}

function renderBuyerAddressesList() {
  const container = document.getElementById("buyer-addresses-list");
  if (!container) return;

  if (buyerAddresses.length === 0) {
    container.innerHTML = `<p style="font-size:0.8rem; color:var(--text-muted); text-align:center; padding:1rem">Belum ada alamat tersimpan.</p>`;
    return;
  }

  container.innerHTML = buyerAddresses.map(a => `
    <div class="address-item-card ${a.is_default ? 'is-default' : ''}">
      <div style="display:flex; justify-content:space-between; align-items:center">
        <div style="display:flex; align-items:center; gap:0.4rem">
          <span class="address-badge-label">${a.label || 'Rumah'}</span>
          ${a.is_default ? '<span class="address-badge-default">UTAMA / DEFAULT</span>' : ''}
        </div>
        <div style="display:flex; gap:0.4rem">
          ${!a.is_default ? `<button type="button" class="btn-sm-action" onclick="handleSetDefaultAddress('${a.id}')">Jadikan Utama</button>` : ''}
          <button type="button" class="btn-sm-action" style="color:var(--rose); border-color:rgba(239,68,68,0.3)" onclick="handleDeleteAddress('${a.id}')">Hapus</button>
        </div>
      </div>
      <div style="font-size:0.85rem; font-weight:700; color:var(--text-heading); margin-top:0.2rem">
        ${a.recipient_name} <span style="font-weight:400; font-size:0.8rem; color:var(--text-muted)">(${a.phone_number})</span>
      </div>
      <div style="font-size:0.8rem; color:var(--text-muted)">
        ${a.street_address}, Kec. ${a.subdistrict}, ${a.city}, ${a.province} ${a.postal_code}
      </div>
    </div>
  `).join('');
}

function openAddAddressModal() {
  const modal = document.getElementById("buyer-address-modal");
  const form = document.getElementById("buyer-address-form");
  if (form) form.reset();
  const idInput = document.getElementById("addr-id");
  if (idInput) idInput.value = "";
  if (activeBuyer && activeBuyer.full_name) {
    const recInput = document.getElementById("addr-recipient-name");
    if (recInput) recInput.value = activeBuyer.full_name;
  }
  if (activeBuyer && activeBuyer.phone_number) {
    const phoneInput = document.getElementById("addr-phone");
    if (phoneInput) phoneInput.value = activeBuyer.phone_number;
  }
  const defCheck = document.getElementById("addr-is-default");
  if (defCheck) defCheck.checked = buyerAddresses.length === 0;
  if (modal) modal.style.display = "flex";
}

function closeBuyerAddressModal() {
  const modal = document.getElementById("buyer-address-modal");
  if (modal) modal.style.display = "none";
}

async function handleSaveBuyerAddress(e) {
  if (e) e.preventDefault();
  const id = document.getElementById("addr-id").value;
  const label = document.getElementById("addr-label").value.trim() || "Rumah";
  const recipient_name = document.getElementById("addr-recipient-name").value.trim();
  const phone_number = document.getElementById("addr-phone").value.trim();
  const street_address = document.getElementById("addr-street").value.trim();
  const subdistrict = document.getElementById("addr-subdistrict").value.trim();
  const city = document.getElementById("addr-city").value.trim();
  const province = document.getElementById("addr-province").value.trim();
  const postal_code = document.getElementById("addr-postal-code").value.trim();
  const is_default = document.getElementById("addr-is-default").checked;

  const payload = {
    label: label ? label : undefined,
    recipient_name,
    phone_number,
    street_address,
    subdistrict,
    city,
    province,
    postal_code,
    set_as_default: is_default
  };

  try {
    const url = id ? `/api/v1/buyer/addresses/${id}` : "/api/v1/buyer/addresses";
    const method = id ? "PUT" : "POST";
    const res = await buyerAuthFetch(url, {
      method,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });

    if (res.ok) {
      closeBuyerAddressModal();
      await fetchBuyerAddresses();
      updateCartUI();
      alert("✅ Alamat pengiriman berhasil disimpan!");
    } else {
      const err = await res.json();
      alert(`Gagal menyimpan alamat: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (e) {
    console.error("Save address error:", e);
    alert(`Error: ${e.message}`);
  }
}

async function handleSetDefaultAddress(id) {
  try {
    const res = await buyerAuthFetch(`/api/v1/buyer/addresses/${id}/default`, { method: "POST" });
    if (res.ok) {
      selectedAddressId = id;
      await fetchBuyerAddresses();
      updateCartUI();
    } else {
      const err = await res.json();
      alert(`Gagal mengatur alamat default: ${err.message || err.error}`);
    }
  } catch (e) {
    console.error("Set default address error:", e);
  }
}

async function handleDeleteAddress(id) {
  if (!confirm("Hapus alamat ini dari buku alamat Anda?")) return;
  try {
    const res = await buyerAuthFetch(`/api/v1/buyer/addresses/${id}`, { method: "DELETE" });
    if (res.ok) {
      await fetchBuyerAddresses();
      updateCartUI();
    } else {
      const err = await res.json();
      alert(`Gagal menghapus alamat: ${err.message || err.error}`);
    }
  } catch (e) {
    console.error("Delete address error:", e);
  }
}

// Flash Sale Countdown Timer Logic
function startFlashSaleTimer() {
  let seconds = 3 * 3600 + 45 * 60 + 12; // 03:45:12
  const hoursEl = document.getElementById('timer-hours');
  const minsEl = document.getElementById('timer-mins');
  const secsEl = document.getElementById('timer-secs');

  if (!hoursEl || !minsEl || !secsEl) return;

  setInterval(() => {
    if (seconds <= 0) seconds = 24 * 3600;
    seconds--;

    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;

    hoursEl.innerText = String(h).padStart(2, '0');
    minsEl.innerText = String(m).padStart(2, '0');
    secsEl.innerText = String(s).padStart(2, '0');
  }, 1000);
}

// Category Pills Generator
function renderCategoryPills() {
  const container = document.getElementById('category-bar');
  if (!container) return;

  const categories = ["ALL", ...new Set(catalog.map(p => p.category))];

  container.innerHTML = categories.map(cat => `
    <button class="category-pill ${cat === activeCategory ? 'active' : ''}" onclick="filterCategory('${cat}')">
      ${cat === 'ALL' ? '🔥 Semua Produk' : cat}
    </button>
  `).join('');
}

function filterCategory(cat) {
  activeCategory = cat;
  renderCategoryPills();
  renderCatalog();
}

function setupSearchListener() {
  const input = document.getElementById('search-input');
  if (input) {
    input.addEventListener('input', (e) => {
      searchQuery = e.target.value.toLowerCase().trim();
      renderCatalog();
    });
  }
}

// Render Products Grid (Shopee-Style Card)
function renderCatalog() {
  const grid = document.getElementById('product-grid');
  if (!grid) return;

  let filtered = catalog;
  if (activeCategory !== 'ALL') {
    filtered = filtered.filter(p => p.category === activeCategory);
  }

  if (searchQuery) {
    filtered = filtered.filter(p =>
      p.name.toLowerCase().includes(searchQuery) ||
      p.description.toLowerCase().includes(searchQuery) ||
      p.sku.toLowerCase().includes(searchQuery)
    );
  }

  if (filtered.length === 0) {
    grid.innerHTML = '<div style="grid-column:1/-1; text-align:center; padding:3rem; color:var(--text-muted)">Produk tidak ditemukan.</div>';
    return;
  }

  grid.innerHTML = filtered.map(p => `
    <div class="product-card">
      <span class="discount-badge">OFF 15%</span>
      <img src="${p.image_url}" alt="${p.name}" class="product-img" loading="lazy">
      <div class="product-info">
        <span class="product-category">${p.category}</span>
        <h4 class="product-name">${p.name}</h4>
        <p class="product-desc">${p.description || ""}</p>
        <div class="product-rating">
          ★ ★ ★ ★ ★ <span style="color:var(--text-muted); font-size:0.7rem">(4.9 | 1.2k Terjual)</span>
        </div>
        <div class="product-footer">
          <span class="product-price">Rp ${p.price.toLocaleString('id-ID')}</span>
          <button class="btn-add-cart" onclick="addToCart('${p.id}')">+ Beli</button>
        </div>
      </div>
    </div>
  `).join('');
}

// Shopping Cart State & UI Controls
function addToCart(productId) {
  const item = catalog.find(p => p.id === productId);
  if (!item) return;

  const existing = cart.find(c => c.product_id === productId);
  if (existing) {
    existing.quantity += 1;
  } else {
    cart.push({ product_id: productId, name: item.name, price: item.price, quantity: 1 });
  }

  updateCartUI();
}

function updateQuantity(productId, delta) {
  const item = cart.find(c => c.product_id === productId);
  if (!item) return;

  item.quantity += delta;
  if (item.quantity <= 0) {
    cart = cart.filter(c => c.product_id !== productId);
  }
  updateCartUI();
}

function updateCartUI() {
  const container = document.getElementById('cart-items-container');
  const checkoutSection = document.getElementById('checkout-section');
  const unauthBox = document.getElementById('checkout-unauth-box');
  const unverifiedBox = document.getElementById('checkout-unverified-box');
  const confirmedForm = document.getElementById('checkout-confirmed-form');
  const badge = document.getElementById('cart-count');

  const count = cart.reduce((acc, i) => acc + i.quantity, 0);
  if (badge) badge.innerText = count;

  if (!container || !checkoutSection) return;

  if (cart.length === 0) {
    container.innerHTML = '<p style="color:var(--text-muted); text-align:center; padding:1.5rem">Keranjang belanja Anda kosong.</p>';
    checkoutSection.style.display = 'none';
    return;
  }

  let total = 0;
  container.innerHTML = cart.map(i => {
    const itemTotal = i.price * i.quantity;
    total += itemTotal;
    return `
      <div class="cart-item">
        <div>
          <strong style="color:#fff">${i.name}</strong>
          <div style="font-size:0.8rem; color:var(--text-muted)">Rp ${i.price.toLocaleString('id-ID')}</div>
          <div class="cart-qty-controls">
            <button class="btn-qty" onclick="updateQuantity('${i.product_id}', -1)">-</button>
            <span style="font-size:0.85rem; font-weight:700; padding:0 0.4rem">${i.quantity}</span>
            <button class="btn-qty" onclick="updateQuantity('${i.product_id}', 1)">+</button>
          </div>
        </div>
        <span style="font-weight:700; color:var(--shopee-orange)">Rp ${itemTotal.toLocaleString('id-ID')}</span>
      </div>
    `;
  }).join('') + `
    <div style="display:flex; justify-content:space-between; margin-top:1rem; font-size:1.1rem; font-weight:700">
      <span style="color:#fff">Total Pembayaran:</span>
      <span style="color:var(--shopee-orange)">Rp ${total.toLocaleString('id-ID')}</span>
    </div>
  `;

  checkoutSection.style.display = 'block';

  // Evaluate Buyer Authentication & Phone Verification Gates
  if (!buyerToken || !activeBuyer) {
    if (unauthBox) unauthBox.style.display = 'block';
    if (unverifiedBox) unverifiedBox.style.display = 'none';
    if (confirmedForm) confirmedForm.style.display = 'none';
  } else if (!activeBuyer.phone_verified) {
    if (unauthBox) unauthBox.style.display = 'none';
    if (unverifiedBox) unverifiedBox.style.display = 'block';
    if (confirmedForm) confirmedForm.style.display = 'none';
  } else {
    if (unauthBox) unauthBox.style.display = 'none';
    if (unverifiedBox) unverifiedBox.style.display = 'none';
    if (confirmedForm) confirmedForm.style.display = 'block';
    renderConfirmedAddressInCheckout();
  }
}

function renderConfirmedAddressInCheckout() {
  const card = document.getElementById("confirmed-address-card");
  if (!card) return;

  if (buyerAddresses.length === 0) {
    card.innerHTML = `
      <div style="text-align:center; padding:0.5rem">
        <p style="font-size:0.85rem; color:var(--text-muted); margin-bottom:0.75rem">Belum ada alamat tersimpan.</p>
        <button type="button" class="btn-checkout" style="font-size:0.85rem; padding:0.4rem 1rem" onclick="openAddAddressModal()">+ Tambah Alamat Pengiriman</button>
      </div>
    `;
    return;
  }

  const currentAddr = buyerAddresses.find(a => a.id === selectedAddressId) || buyerAddresses[0];
  selectedAddressId = currentAddr.id;

  card.innerHTML = `
    <div>
      <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.5rem">
        <span class="addr-title">👤 ${currentAddr.recipient_name} <span class="address-badge-label">${currentAddr.label || 'Rumah'}</span></span>
        <select id="addr-switch-select" class="form-control" style="width:auto; font-size:0.75rem; padding:0.2rem 0.5rem" onchange="switchCheckoutAddress(this.value)">
          ${buyerAddresses.map(a => `<option value="${a.id}" ${a.id === selectedAddressId ? 'selected' : ''}>${a.label || 'Alamat'}: ${a.recipient_name} (${a.city})</option>`).join('')}
        </select>
      </div>
      <div class="addr-phone">📞 ${currentAddr.phone_number} (Terverifikasi OTP 🟢)</div>
      <div class="addr-text">
        📍 ${currentAddr.street_address}, Kec. ${currentAddr.subdistrict}, ${currentAddr.city}, ${currentAddr.province}
      </div>
      <div style="font-size:0.8rem; color:var(--cyan); font-family:'JetBrains Mono'; margin-top:0.25rem">
        Kode Pos: ${currentAddr.postal_code}
      </div>
    </div>
  `;
}

function switchCheckoutAddress(addrId) {
  selectedAddressId = addrId;
  renderConfirmedAddressInCheckout();
}

async function handleProcessCheckout(e) {
  if (e) e.preventDefault();

  if (cart.length === 0) {
    alert("Keranjang belanja kosong.");
    return;
  }

  if (!buyerToken || !activeBuyer) {
    openBuyerLoginModal();
    return;
  }

  if (!activeBuyer.phone_verified) {
    openOtpModal();
    return;
  }

  const selectedAddr = buyerAddresses.find(a => a.id === selectedAddressId);
  if (!selectedAddr) {
    alert("Harap tambahkan alamat pengiriman terlebih dahulu.");
    openAddAddressModal();
    return;
  }

  const items = cart.map(i => ({ product_id: i.product_id, quantity: i.quantity }));

  const payload = {
    address_id: selectedAddressId,
    items: items
  };

  const btnSubmit = document.getElementById("btn-submit-order");
  if (btnSubmit) {
    btnSubmit.disabled = true;
    btnSubmit.innerText = "Memproses Pesanan...";
  }

  try {
    const res = await buyerAuthFetch("/api/v1/orders", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });

    if (res.ok) {
      const orderData = await res.json();
      cart = [];
      updateCartUI();
      closeCart();
      showToast(`🎉 Pesanan berhasil dibuat! Menyiapkan gerbang pembayaran...`, 'success', 4000);
      await initiateMidtransPayment(orderData.id);
    } else {
      const err = await res.json();
      showToast(`Checkout Gagal: ${err.message || err.error || JSON.stringify(err)}`, 'error');
    }
  } catch (err) {
    console.error("Checkout order error:", err);
    showToast(`Error saat membuat order: ${err.message}`, 'error');
  } finally {
    if (btnSubmit) {
      btnSubmit.disabled = false;
      btnSubmit.innerText = "🚀 Konfirmasi & Pesan Sekarang";
    }
  }
}

async function initiateMidtransPayment(orderId) {
  try {
    const res = await buyerAuthFetch("/api/v1/payments", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ order_id: orderId })
    });

    if (!res.ok) {
      const err = await res.json();
      showToast(`Gagal memuat pembayaran: ${err.message || err.error || 'Terjadi kesalahan'}`, 'error');
      return;
    }

    const payment = await res.json();

    if (window.snap && typeof window.snap.pay === "function" && payment.snap_token) {
      window.snap.pay(payment.snap_token, {
        onSuccess: function(result) {
          showToast('🎉 Pembayaran Berhasil Dikonfirmasi!', 'success');
          console.log('Payment success:', result);
        },
        onPending: function(result) {
          showToast('⏳ Menunggu Pembayaran. Silakan selesaikan transaksi Anda.', 'warning');
          console.log('Payment pending:', result);
        },
        onError: function(result) {
          showToast('❌ Pembayaran Gagal.', 'error');
          console.error('Payment error:', result);
        },
        onClose: function() {
          showToast('Jendela pembayaran ditutup. Anda dapat membayar nanti.', 'info');
        }
      });
    } else if (payment.snap_redirect_url) {
      showToast('Membuka halaman pembayaran Midtrans Snap...', 'info');
      window.open(payment.snap_redirect_url, '_blank');
    } else {
      showToast("Pesanan disimpan. Menunggu konfirmasi pembayaran.", "info");
    }
  } catch (err) {
    console.error("Initiate payment error:", err);
    showToast(`Error pembayaran: ${err.message}`, 'error');
  }
}

function openCart() {
  const modal = document.getElementById('cart-modal');
  if (modal) modal.style.display = 'flex';
  updateCartUI();
}

function closeCart() {
  const modal = document.getElementById('cart-modal');
  if (modal) modal.style.display = 'none';
}

// --- HYBRID CUSTOMER SUPPORT & CHAT MODULE (WHATSAPP + IN-APP LIVE CHAT - OPTION 3) ---

function toggleChatPopup(forceState) {
  const card = document.getElementById("chat-popup-card");
  if (!card) return;

  if (typeof forceState === "boolean") {
    card.style.display = forceState ? "block" : "none";
  } else {
    card.style.display = card.style.display === "block" ? "none" : "block";
  }

  if (card.style.display === "none") {
    closeInAppChatWindow();
  }
}

function updateFloatingChatWidget() {
  const storeTitle = document.getElementById("chat-store-title");
  if (storeTitle && storeInfo) {
    storeTitle.innerText = `${storeInfo.store_name || "CS Toko"}`;
  }

  const memberBadge = document.getElementById("chat-member-badge");
  const memberName = document.getElementById("chat-member-name");
  const memberStatus = document.getElementById("chat-member-status");
  const fabLabel = document.getElementById("chat-fab-label");
  const fabBtn = document.getElementById("btn-floating-chat");

  if (activeBuyer && buyerToken) {
    if (memberBadge) memberBadge.style.display = "block";
    if (memberName) memberName.innerText = `${activeBuyer.full_name} (${activeBuyer.email})`;
    if (memberStatus) memberStatus.innerHTML = `● Sapaan Member: <strong style="color:#fff">${activeBuyer.full_name}</strong>`;
    if (fabLabel) fabLabel.innerText = "💬 Chat Penjual";
    if (fabBtn) fabBtn.classList.add("fab-member-active");
    const waBtnTitle = document.getElementById("wa-btn-title");
    const waBtnSubtitle = document.getElementById("wa-btn-subtitle");
    if (waBtnTitle) waBtnTitle.innerText = "Chat Langsung via WhatsApp";
    if (waBtnSubtitle) waBtnSubtitle.innerText = "Member Aktif ⚡ Terhubung Langsung ke Penjual";
  } else {
    if (memberBadge) memberBadge.style.display = "none";
    if (memberStatus) memberStatus.innerText = "● CS Online & Siap Membantu";
    if (fabLabel) fabLabel.innerText = "Chat Penjual";
    if (fabBtn) fabBtn.classList.remove("fab-member-active");
    const waBtnTitle = document.getElementById("wa-btn-title");
    const waBtnSubtitle = document.getElementById("wa-btn-subtitle");
    if (waBtnTitle) waBtnTitle.innerText = "Chat via WhatsApp (Khusus Member)";
    if (waBtnSubtitle) waBtnSubtitle.innerText = "🔒 Masuk / daftar untuk chat langsung";
  }
}

function handleDirectWhatsAppChat() {
  // WhatsApp is ONLY accessible after registered or logged in
  if (!activeBuyer || !buyerToken) {
    toggleChatPopup(false);
    openBuyerLoginModal();
    const contextNotice = document.getElementById("buyer-login-context-notice");
    if (contextNotice) {
      contextNotice.style.display = "block";
      contextNotice.innerHTML = "🔒 <strong>Akses WhatsApp Khusus Member:</strong> Silakan masuk atau daftar akun terlebih dahulu untuk menghubungi Penjual via WhatsApp. Anda juga dapat menggunakan <em>Live Chat Toko</em> secara langsung tanpa login.";
    }
    showToast("Fitur WhatsApp khusus untuk pembeli yang sudah terdaftar. Silakan masuk atau buat akun.", "warning");
    return;
  }

  let rawNum = storeWhatsAppNumber || (storeInfo && storeInfo.whatsapp_number) || "085810007735";
  let cleanNum = rawNum.replace(/[^0-9]/g, "");
  if (cleanNum === "6281234567890" || cleanNum === "081234567890" || !cleanNum) {
    cleanNum = "6285810007735";
  } else if (cleanNum.startsWith("08")) {
    cleanNum = "628" + cleanNum.substring(2);
  }
  if (!cleanNum) cleanNum = "6285810007735";

  const storeName = (storeInfo && storeInfo.store_name) || "AURA Storefront";
  const buyerIdSnippet = activeBuyer.id ? activeBuyer.id.substring(0, 8) : "-";
  const phoneStr = activeBuyer.phone_number ? ` (HP: ${activeBuyer.phone_number})` : "";
  const message = `Halo Admin ${storeName}, saya ${activeBuyer.full_name}${phoneStr} (Member ID: ${buyerIdSnippet}).\n\nSaya adalah pembeli terdaftar dan ingin berkonsultasi mengenai produk / pesanan saya di toko.`;

  toggleChatPopup(false);

  const waUrl = `https://wa.me/${cleanNum}?text=${encodeURIComponent(message)}`;
  window.open(waUrl, "_blank");
}

function openInAppChatWindow() {
  // Free access for everyone (guests and logged-in members)!
  toggleChatPopup(false);
  const win = document.getElementById("inapp-chat-window");
  if (win) win.style.display = "flex";

  if (inAppChatMessages.length === 0) {
    inAppChatMessages.push({
      sender: "system",
      text: "Sesi Live Chat dimulai. Terhubung ke Customer Service Toko.",
      time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    });

    const storeName = (storeInfo && storeInfo.store_name) || "CS Toko";
    if (activeBuyer && buyerToken) {
      inAppChatMessages.push({
        sender: "seller",
        text: `Halo ${activeBuyer.full_name}! 👋 Ada produk atau pesanan yang bisa kami bantu? Anda juga dapat menggunakan WhatsApp untuk respon instan.`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
      });
    } else {
      inAppChatMessages.push({
        sender: "seller",
        text: `Halo Pengunjung! 👋 Selamat datang di ${storeName}. Silakan tanyakan seputar produk atau pesanan Anda di sini. Tim kami siap membantu.`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
      });
    }
  }

  renderInAppChatMessages();
}

function closeInAppChatWindow() {
  const win = document.getElementById("inapp-chat-window");
  if (win) win.style.display = "none";
}

function renderInAppChatMessages() {
  const container = document.getElementById("inapp-chat-messages");
  if (!container) return;

  container.innerHTML = inAppChatMessages.map(m => {
    const senderClass = m.sender; // 'buyer', 'seller', 'system'
    if (senderClass === "system") {
      return `<div class="chat-bubble system">${m.text}</div>`;
    }
    return `
      <div class="chat-bubble ${senderClass}">
        <div>${m.text}</div>
        <div class="chat-bubble-time">${m.time}</div>
      </div>
    `;
  }).join("");

  container.scrollTop = container.scrollHeight;
}

function sendInAppChatMessage(e) {
  if (e) e.preventDefault();
  const input = document.getElementById("inapp-chat-input");
  if (!input) return;
  const text = input.value.trim();
  if (!text) return;

  const nowTime = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

  inAppChatMessages.push({
    sender: "buyer",
    text: text,
    time: nowTime
  });

  input.value = "";
  renderInAppChatMessages();

  // Simulate seller response via Live Chat engine
  setTimeout(() => {
    let replyText = "";
    if (activeBuyer && buyerToken) {
      const buyerName = activeBuyer.full_name || "Kak";
      replyText = `Baik ${buyerName}, pesan Anda telah tercatat di antrean live chat kami. Karena Anda sudah login sebagai member, Anda juga dapat membuka chat WhatsApp Penjual jika membutuhkan respon kilat.`;
    } else {
      replyText = `Terima kasih atas pesan Anda! Tim kami telah menerima pertanyaan Anda. Catatan: Untuk menghubungi Penjual langsung via WhatsApp atau melacak riwayat pesanan, silakan masuk / buat akun terlebih dahulu.`;
    }
    inAppChatMessages.push({
      sender: "seller",
      text: replyText,
      time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    });
    renderInAppChatMessages();
  }, 1000);
}

// --- BUYER ORDERS HISTORY & WORKFLOW ---
function escapeHtml(str) {
  if (!str) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

function openBuyerOrdersModal() {
  if (!buyerToken) {
    alert("Silakan masuk terlebih dahulu untuk melihat riwayat pesanan Anda.");
    openBuyerLoginModal();
    return;
  }
  const modal = document.getElementById("buyer-orders-modal");
  if (modal) modal.style.display = "flex";
  fetchBuyerOrders();
}

function closeBuyerOrdersModal() {
  const modal = document.getElementById("buyer-orders-modal");
  if (modal) modal.style.display = "none";
}

function getBuyerOrderStatusBadge(status) {
  const s = (status || "").toLowerCase();
  switch (s) {
    case "pending":
      return `<span class="buyer-order-status-badge buyer-status-pending">⏳ Menunggu Pembayaran</span>`;
    case "paid":
      return `<span class="buyer-order-status-badge buyer-status-paid">💳 Pembayaran Berhasil</span>`;
    case "processing":
      return `<span class="buyer-order-status-badge buyer-status-processing">⚙️ Sedang Dikemas</span>`;
    case "shipped":
      return `<span class="buyer-order-status-badge buyer-status-shipped">🚚 Sedang Dikirim</span>`;
    case "delivered":
      return `<span class="buyer-order-status-badge buyer-status-delivered">📬 Pesanan Sampai</span>`;
    case "completed":
      return `<span class="buyer-order-status-badge buyer-status-completed">✅ Transaksi Selesai</span>`;
    case "cancelled":
      return `<span class="buyer-order-status-badge buyer-status-cancelled">❌ Dibatalkan</span>`;
    case "returned":
    case "return_requested":
      return `<span class="buyer-order-status-badge buyer-status-returned">↩️ Retur / Pengembalian</span>`;
    default:
      return `<span class="buyer-order-status-badge">${escapeHtml(status)}</span>`;
  }
}

async function fetchBuyerOrders() {
  const container = document.getElementById("buyer-orders-content");
  if (!container) return;
  if (!buyerToken) return;

  container.innerHTML = `<p style="text-align:center; color:var(--text-muted); padding:1.5rem">Memuat data pesanan...</p>`;

  try {
    const res = await fetch("/api/v1/buyer/orders", {
      headers: {
        "Authorization": `Bearer ${buyerToken}`
      }
    });

    if (!res.ok) {
      if (res.status === 401) {
        logoutBuyer();
        closeBuyerOrdersModal();
        alert("Sesi masuk Anda telah berakhir. Silakan login kembali.");
        return;
      }
      const err = await res.json();
      container.innerHTML = `<p style="color:var(--rose); text-align:center; padding:1.5rem">Gagal memuat pesanan: ${escapeHtml(err.message || err.error || "Unknown error")}</p>`;
      return;
    }

    const orders = await res.json();
    if (!orders || orders.length === 0) {
      container.innerHTML = `
        <div style="text-align:center; padding:2.5rem 1rem; color:var(--text-muted)">
          <div style="font-size:2.5rem; margin-bottom:0.5rem">🛍️</div>
          <p style="font-weight:600; color:var(--text-heading); margin-bottom:0.3rem">Belum Ada Pesanan</p>
          <p style="font-size:0.85rem">Anda belum melakukan transaksi belanja apapun.</p>
        </div>
      `;
      return;
    }

    container.innerHTML = orders.map(o => {
      const s = (o.status || "").toLowerCase();
      let actionsHtml = "";

      if (s === "pending") {
        actionsHtml = `
          <div style="display:flex; gap:0.5rem; align-items:center">
            <button class="btn-order-cancel" onclick="handleBuyerCancelOrder('${o.id}')">❌ Batalkan Pesanan</button>
          </div>
        `;
      } else if (s === "shipped") {
        actionsHtml = `
          <div style="display:flex; gap:0.5rem; align-items:center">
            <button class="btn-order-confirm" onclick="handleBuyerConfirmDelivery('${o.id}')">✅ Konfirmasi Barang Diterima</button>
          </div>
        `;
      }

      let trackingHtml = "";
      if (o.tracking_number) {
        trackingHtml = `<div class="buyer-order-tracking">🚚 No. Resi: <strong>${escapeHtml(o.tracking_number)}</strong></div>`;
      }

      let cancelInfoHtml = "";
      if (s === "cancelled" && o.cancel_reason) {
        cancelInfoHtml = `<div style="font-size:0.75rem; color:var(--rose); margin-top:0.2rem">Alasan: ${escapeHtml(o.cancel_reason)}</div>`;
      }

      const orderDate = new Date(o.created_at).toLocaleString("id-ID", {
        day: "numeric",
        month: "short",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit"
      });

      return `
        <div class="buyer-order-card">
          <div class="buyer-order-header">
            <div>
              <span class="buyer-order-id">Order ID: #${escapeHtml(o.id.substring(0,8))}</span>
              <div style="font-size:0.75rem; color:var(--text-muted); margin-top:2px">${orderDate}</div>
            </div>
            <div>${getBuyerOrderStatusBadge(o.status)}</div>
          </div>
          <div class="buyer-order-body">
            <div>
              <div style="font-size:0.8rem; color:var(--text-muted)">Total Pembayaran:</div>
              <div class="buyer-order-amount">Rp ${o.total_amount.toLocaleString("id-ID")}</div>
              ${cancelInfoHtml}
            </div>
            <div>
              ${trackingHtml}
            </div>
          </div>
          ${actionsHtml ? `<div class="buyer-order-footer">${actionsHtml}</div>` : ""}
        </div>
      `;
    }).join("");

  } catch (e) {
    console.error("fetchBuyerOrders error:", e);
    container.innerHTML = `<p style="color:var(--rose); text-align:center; padding:1.5rem">Error: ${escapeHtml(e.message)}</p>`;
  }
}

async function handleBuyerCancelOrder(orderId) {
  const confirmCancel = confirm("Apakah Anda yakin ingin membatalkan pesanan ini?");
  if (!confirmCancel) return;

  const reason = prompt("Alasan pembatalan (opsional):", "Ingin mengubah rincian pesanan");
  if (reason === null) return;

  try {
    const res = await fetch(`/api/v1/orders/${orderId}/cancel`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${buyerToken}`
      },
      body: JSON.stringify({ reason: reason || "Dibatalkan oleh pembeli" })
    });

    if (res.ok) {
      alert("✅ Pesanan berhasil dibatalkan.");
      fetchBuyerOrders();
    } else {
      const err = await res.json();
      alert(`❌ Gagal membatalkan pesanan: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (e) {
    alert(`❌ Error: ${e.message}`);
  }
}

async function handleBuyerConfirmDelivery(orderId) {
  const confirmReceived = confirm("Konfirmasi bahwa Anda telah menerima barang pesanan ini dalam kondisi baik?");
  if (!confirmReceived) return;

  try {
    const res = await fetch(`/api/v1/orders/${orderId}/confirm-delivery`, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${buyerToken}`
      }
    });

    if (res.ok) {
      alert("🎉 Terima kasih! Pesanan telah dikonfirmasi diterima.");
      fetchBuyerOrders();
    } else {
      const err = await res.json();
      alert(`❌ Gagal konfirmasi pesanan: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (e) {
    alert(`❌ Error: ${e.message}`);
  }
}

// Window Exports
window.openBuyerLoginModal = openBuyerLoginModal;
window.closeBuyerLoginModal = closeBuyerLoginModal;
window.openBuyerOrdersModal = openBuyerOrdersModal;
window.closeBuyerOrdersModal = closeBuyerOrdersModal;
window.fetchBuyerOrders = fetchBuyerOrders;
window.handleBuyerCancelOrder = handleBuyerCancelOrder;
window.handleBuyerConfirmDelivery = handleBuyerConfirmDelivery;
window.renderGoogleSignInButton = renderGoogleSignInButton;
window.handleGoogleCredentialResponse = handleGoogleCredentialResponse;
window.logoutBuyer = logoutBuyer;
window.openOtpModal = openOtpModal;
window.closeBuyerOtpModal = closeBuyerOtpModal;
window.handleRequestOtp = handleRequestOtp;
window.handleVerifyOtp = handleVerifyOtp;
window.handleResendOtp = handleResendOtp;
window.openAddAddressModal = openAddAddressModal;
window.closeBuyerAddressModal = closeBuyerAddressModal;
window.handleSaveBuyerAddress = handleSaveBuyerAddress;
window.handleSetDefaultAddress = handleSetDefaultAddress;
window.handleDeleteAddress = handleDeleteAddress;
window.switchCheckoutAddress = switchCheckoutAddress;
window.handleProcessCheckout = handleProcessCheckout;
window.initiateMidtransPayment = initiateMidtransPayment;
window.openCart = openCart;
window.closeCart = closeCart;
window.toggleChatPopup = toggleChatPopup;
window.handleDirectWhatsAppChat = handleDirectWhatsAppChat;
window.openInAppChatWindow = openInAppChatWindow;
window.closeInAppChatWindow = closeInAppChatWindow;
window.sendInAppChatMessage = sendInAppChatMessage;
window.updateFloatingChatWidget = updateFloatingChatWidget;

document.addEventListener("DOMContentLoaded", () => {
  initStore();
});
