/* ==========================================================================
   STOREFRONT BUYER AUTH, GOOGLE OAUTH & OTP ENGINE
   File: /crates/web/static/store/store-auth.js
   ========================================================================== */

async function buyerAuthFetch(url, options = {}) {
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  if (!token) {
    throw new Error("Authentication required. Please log in.");
  }
  options.headers = options.headers || {};
  options.headers["Authorization"] = `Bearer ${token}`;
  const res = await fetch(url, options);
  if (res.status === 401 || res.status === 403) {
    console.warn("Buyer token invalid or expired. Logging out...");
    logoutBuyer();
  }
  return res;
}

async function checkBuyerSession() {
  const token = localStorage.getItem("program1_buyer_token") || null;
  if (window.StoreState) window.StoreState.buyerToken = token;
  window.buyerToken = token;

  const saved = localStorage.getItem("program1_buyer_user");
  if (token && saved) {
    try {
      const parsed = JSON.parse(saved);
      if (window.StoreState) window.StoreState.activeBuyer = parsed;
      window.activeBuyer = parsed;
      renderBuyerHeaderState();
      await syncBuyerProfile();
    } catch (e) {
      logoutBuyer();
    }
  } else {
    renderBuyerHeaderState();
  }
}

async function syncBuyerProfile() {
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  if (!token) return;
  try {
    const res = await buyerAuthFetch("/api/v1/buyer/profile");
    if (res.ok) {
      const buyer = await res.json();
      if (window.StoreState) window.StoreState.activeBuyer = buyer;
      window.activeBuyer = buyer;
      localStorage.setItem("program1_buyer_user", JSON.stringify(buyer));
      renderBuyerHeaderState();
      if (typeof fetchBuyerAddresses === "function") {
        await fetchBuyerAddresses();
      }
    }
  } catch (e) {
    console.error("Error syncing buyer profile:", e);
  }
}

function renderBuyerHeaderState() {
  const buyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;

  const btnLogin = document.getElementById("btn-buyer-login");
  const badgeProfile = document.getElementById("buyer-profile-badge");
  const btnOrders = document.getElementById("btn-buyer-orders");
  const avatarEl = document.getElementById("buyer-avatar");
  const nameLabelEl = document.getElementById("buyer-name-label");

  if (buyer && token) {
    if (btnLogin) btnLogin.style.display = "none";
    if (badgeProfile) badgeProfile.style.display = "flex";
    if (btnOrders) btnOrders.style.display = "inline-flex";
    if (avatarEl) avatarEl.innerText = (buyer.full_name || "P").charAt(0).toUpperCase();
    if (nameLabelEl) nameLabelEl.innerText = buyer.full_name || "Pembeli";
  } else {
    if (btnLogin) btnLogin.style.display = "block";
    if (badgeProfile) badgeProfile.style.display = "none";
    if (btnOrders) btnOrders.style.display = "none";
  }
  if (typeof updateFloatingChatWidget === "function") {
    updateFloatingChatWidget();
  }
  if (typeof updateNotificationBadge === "function") {
    updateNotificationBadge();
  }
}

function openBuyerLoginModal() {
  const buyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;

  const modal = document.getElementById("buyer-login-modal");
  const loggedView = document.getElementById("buyer-logged-in-view");
  const unauthView = document.getElementById("buyer-unauthenticated-view");

  if (buyer && token) {
    if (loggedView) loggedView.style.display = "block";
    if (unauthView) unauthView.style.display = "none";
    const avatarEl = document.getElementById("modal-buyer-avatar");
    const nameEl = document.getElementById("modal-buyer-name");
    const emailEl = document.getElementById("modal-buyer-email");
    if (avatarEl) avatarEl.innerText = (buyer.full_name || "P").charAt(0).toUpperCase();
    if (nameEl) nameEl.innerText = buyer.full_name || "Pembeli";
    if (emailEl) emailEl.innerText = buyer.email || "-";

    const phoneBadge = document.getElementById("buyer-phone-status-badge");
    if (phoneBadge) {
      if (buyer.phone_verified && buyer.phone_number) {
        phoneBadge.innerHTML = `<span style="font-size:0.8rem; color:var(--emerald); font-weight:600">🟢 No. HP Terverifikasi: ${buyer.phone_number}</span>`;
      } else {
        phoneBadge.innerHTML = `
          <div style="display:flex; align-items:center; gap:0.5rem">
            <span style="font-size:0.8rem; color:var(--rose); font-weight:600">🔴 Belum Verifikasi No. HP</span>
            <button class="btn-sm-action" onclick="openOtpModal()">Verifikasi Sekarang</button>
          </div>
        `;
      }
    }

    if (typeof fetchBuyerAddresses === "function") {
      fetchBuyerAddresses();
    }
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
      const gid = data.google_client_id || null;
      if (window.StoreState) window.StoreState.googleClientId = gid;
      window.googleClientId = gid;
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
    const token = data.access_token || data.token;
    const buyer = data.buyer;
    if (window.StoreState) {
      window.StoreState.buyerToken = token;
      window.StoreState.activeBuyer = buyer;
    }
    window.buyerToken = token;
    window.activeBuyer = buyer;
    localStorage.setItem("program1_buyer_token", token);
    localStorage.setItem("program1_buyer_user", JSON.stringify(buyer));
    renderBuyerHeaderState();
    closeBuyerLoginModal();

    showToast(`🎉 Selamat datang kembali, ${buyer.full_name}!`, "success");

    if (data.requires_phone_verification || !buyer.phone_verified) {
      openOtpModal();
    } else {
      if (typeof fetchBuyerAddresses === "function") await fetchBuyerAddresses();
      if (typeof updateCartUI === "function") updateCartUI();
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
    const token = data.access_token || data.token;
    const buyer = data.buyer;
    if (window.StoreState) {
      window.StoreState.buyerToken = token;
      window.StoreState.activeBuyer = buyer;
    }
    window.buyerToken = token;
    window.activeBuyer = buyer;
    localStorage.setItem("program1_buyer_token", token);
    localStorage.setItem("program1_buyer_user", JSON.stringify(buyer));
    renderBuyerHeaderState();
    closeBuyerLoginModal();

    showToast(`🎉 Pendaftaran berhasil! Selamat bergabung, ${buyer.full_name}.`, "success");

    if (data.requires_phone_verification || !buyer.phone_verified) {
      openOtpModal();
    } else {
      if (typeof fetchBuyerAddresses === "function") await fetchBuyerAddresses();
      if (typeof updateCartUI === "function") updateCartUI();
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
      const token = data.access_token || data.token;
      const buyer = data.buyer;
      if (window.StoreState) {
        window.StoreState.buyerToken = token;
        window.StoreState.activeBuyer = buyer;
      }
      window.buyerToken = token;
      window.activeBuyer = buyer;
      localStorage.setItem("program1_buyer_token", token);
      localStorage.setItem("program1_buyer_user", JSON.stringify(buyer));
      renderBuyerHeaderState();
      closeBuyerLoginModal();
      showToast(`⚡ Berhasil login instan demo sebagai: ${buyer.full_name}`, "success");
      openOtpModal();
    } else {
      const err = await res.json();
      showToast(`Gagal login demo: ${err.message || err.error}`, "error");
    }
  } catch (e) {
    showToast(`Error: ${e.message}`, "error");
  }
}

function renderGoogleSignInButton() {
  const btnContainer = document.getElementById("google-signin-btn");
  const unconfiguredMsg = document.getElementById("google-signin-unconfigured");
  if (!btnContainer) return;

  const gid = window.StoreState ? window.StoreState.googleClientId : window.googleClientId;

  if (!gid || gid.trim() === "" || gid.includes("your-google-client-id")) {
    if (unconfiguredMsg) {
      unconfiguredMsg.innerText = "Google Sign-In belum dikonfigurasi resmi di server. Silakan daftar / masuk menggunakan form email di atas.";
      unconfiguredMsg.style.display = "block";
    }
    return;
  }

  if (unconfiguredMsg) unconfiguredMsg.style.display = "none";

  if (typeof window.google === "undefined" || !window.google.accounts || !window.google.accounts.id) {
    if (window.StoreState) window.StoreState.gisRenderAttempts++;
    const attempts = window.StoreState ? window.StoreState.gisRenderAttempts : 1;
    if (attempts < 15) {
      setTimeout(renderGoogleSignInButton, 300);
    } else {
      if (unconfiguredMsg) {
        unconfiguredMsg.innerText = "SDK Google Sign-In tidak dapat dimuat. Pastikan jaringan internet aktif.";
        unconfiguredMsg.style.display = "block";
      }
    }
    return;
  }

  if (window.StoreState) window.StoreState.gisRenderAttempts = 0;
  btnContainer.innerHTML = "";

  try {
    window.google.accounts.id.initialize({
      client_id: gid,
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
      const token = data.access_token || data.token;
      const buyer = data.buyer;
      if (window.StoreState) {
        window.StoreState.buyerToken = token;
        window.StoreState.activeBuyer = buyer;
      }
      window.buyerToken = token;
      window.activeBuyer = buyer;
      localStorage.setItem("program1_buyer_token", token);
      localStorage.setItem("program1_buyer_user", JSON.stringify(buyer));
      renderBuyerHeaderState();
      closeBuyerLoginModal();

      showToast(`🎉 Selamat datang, ${buyer.full_name}! Login Google berhasil.`, "success");

      if (data.requires_phone_verification || !buyer.phone_verified) {
        openOtpModal();
      } else {
        if (typeof fetchBuyerAddresses === "function") await fetchBuyerAddresses();
        if (typeof updateCartUI === "function") updateCartUI();
      }
    } else {
      const err = await res.json();
      showToast(`Gagal login dengan Google: ${err.message || err.error || JSON.stringify(err)}`, "error");
    }
  } catch (e) {
    console.error("Google Auth error:", e);
    showToast(`Error: ${e.message}`, "error");
  }
}

function logoutBuyer() {
  if (window.StoreState) {
    window.StoreState.buyerToken = null;
    window.StoreState.activeBuyer = null;
    window.StoreState.buyerAddresses = [];
    window.StoreState.selectedAddressId = null;
  }
  window.buyerToken = null;
  window.activeBuyer = null;
  window.buyerAddresses = [];
  window.selectedAddressId = null;

  localStorage.removeItem("program1_buyer_token");
  localStorage.removeItem("program1_buyer_user");
  renderBuyerHeaderState();
  closeBuyerLoginModal();
  if (typeof updateCartUI === "function") updateCartUI();
}

// --- OTP PHONE VERIFICATION ENGINE ---
function openOtpModal() {
  closeBuyerLoginModal();
  const modal = document.getElementById("buyer-otp-modal");
  const stepPhone = document.getElementById("otp-step-phone");
  const stepVerify = document.getElementById("otp-step-verify");
  if (stepPhone) stepPhone.style.display = "block";
  if (stepVerify) stepVerify.style.display = "none";
  const buyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  if (buyer && buyer.phone_number) {
    const phoneInput = document.getElementById("otp-input-phone");
    if (phoneInput) phoneInput.value = buyer.phone_number;
  }
  if (modal) modal.style.display = "flex";
}

function closeBuyerOtpModal() {
  const modal = document.getElementById("buyer-otp-modal");
  if (modal) modal.style.display = "none";
  const interval = window.StoreState ? window.StoreState.otpTimerInterval : window.otpTimerInterval;
  if (interval) clearInterval(interval);
}

async function handleRequestOtp() {
  const phoneInput = document.getElementById("otp-input-phone");
  const phone = phoneInput ? phoneInput.value.trim() : "";
  if (!phone || phone.length < 8) {
    showToast("Harap masukkan nomor handphone / WhatsApp yang valid (minimal 8 digit).", "warning");
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
      if (window.StoreState) window.StoreState.currentOtpPhone = phone;
      window.currentOtpPhone = phone;
      const sentLabel = document.getElementById("otp-sent-phone-label");
      if (sentLabel) sentLabel.innerText = phone;
      document.getElementById("otp-step-phone").style.display = "none";
      document.getElementById("otp-step-verify").style.display = "block";
      startOtpCountdown(300);

      // Setup WA Admin link
      const waLink = document.getElementById("otp-wa-admin-link");
      if (waLink) {
        const storeWa = window.STORE_WHATSAPP || (window.StoreState ? window.StoreState.storeWhatsAppNumber : "085810007735");
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
        showToast(`📲 Kode OTP telah dikirimkan ke WhatsApp/SMS nomor ${phone}.`, "info");
      }
    } else {
      const err = await res.json();
      showToast(`Gagal mengirim OTP: ${err.message || err.error || JSON.stringify(err)}`, "error");
    }
  } catch (e) {
    console.error("Request OTP error:", e);
    showToast(`Error: ${e.message}`, "error");
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.innerText = "📲 Kirim Kode OTP";
    }
  }
}

function startOtpCountdown(durationSeconds) {
  const currentInterval = window.StoreState ? window.StoreState.otpTimerInterval : window.otpTimerInterval;
  if (currentInterval) clearInterval(currentInterval);

  let seconds = durationSeconds;
  if (window.StoreState) window.StoreState.otpSecondsRemaining = seconds;
  const timerLabel = document.getElementById("otp-timer-label");

  function update() {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    if (timerLabel) {
      timerLabel.innerText = `${m < 10 ? '0' : ''}${m}:${s < 10 ? '0' : ''}${s}`;
    }
    if (seconds <= 0) {
      if (window.StoreState && window.StoreState.otpTimerInterval) clearInterval(window.StoreState.otpTimerInterval);
      if (timerLabel) timerLabel.innerText = "KEDALUWARSA";
    } else {
      seconds--;
      if (window.StoreState) window.StoreState.otpSecondsRemaining = seconds;
    }
  }
  update();
  const timerId = setInterval(update, 1000);
  if (window.StoreState) window.StoreState.otpTimerInterval = timerId;
  window.otpTimerInterval = timerId;
}

async function handleVerifyOtp() {
  const codeInput = document.getElementById("otp-input-code");
  const code = codeInput ? codeInput.value.trim() : "";
  if (!code || code.length !== 6) {
    showToast("Harap masukkan 6 digit kode OTP.", "warning");
    return;
  }

  const btn = document.getElementById("btn-submit-verify-otp");
  if (btn) {
    btn.disabled = true;
    btn.innerText = "Memverifikasi...";
  }

  const phone = window.StoreState ? window.StoreState.currentOtpPhone : window.currentOtpPhone;

  try {
    const res = await buyerAuthFetch("/api/v1/buyer/otp/verify", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ phone_number: phone, code: code })
    });

    if (res.ok) {
      const result = await res.json();
      activeBuyer = (result && result.id) ? result : (result.buyer || result);
      if (window.StoreState) window.StoreState.activeBuyer = activeBuyer;
      window.activeBuyer = activeBuyer;
      localStorage.setItem("program1_buyer_user", JSON.stringify(activeBuyer));

      const interval = window.StoreState ? window.StoreState.otpTimerInterval : window.otpTimerInterval;
      if (interval) clearInterval(interval);

      closeBuyerOtpModal();
      renderBuyerHeaderState();
      showToast("🎉 Nomor HP Anda berhasil diverifikasi! Alamat pengiriman sekarang dapat digunakan.", "success");
      if (typeof fetchBuyerAddresses === "function") await fetchBuyerAddresses();
      if (typeof updateCartUI === "function") updateCartUI();
    } else {
      const err = await res.json();
      showToast(`Verifikasi OTP Gagal: ${err.message || err.error || JSON.stringify(err)}`, "error");
    }
  } catch (e) {
    console.error("Verify OTP error:", e);
    showToast(`Error: ${e.message}`, "error");
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.innerText = "✅ Verifikasi Kode OTP";
    }
  }
}

function handleResendOtp() {
  const interval = window.StoreState ? window.StoreState.otpTimerInterval : window.otpTimerInterval;
  if (interval) clearInterval(interval);
  document.getElementById("otp-step-phone").style.display = "block";
  document.getElementById("otp-step-verify").style.display = "none";
  const codeInput = document.getElementById("otp-input-code");
  if (codeInput) codeInput.value = "";
  const devNotice = document.getElementById("otp-dev-notice");
  if (devNotice) devNotice.style.display = "none";
}

// --- BUYER PROFILE DASHBOARD ENGINE ---
async function renderBuyerProfileDashboard() {
  const buyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  if (!buyer || !token) return;

  const avatarEl = document.getElementById("profile-avatar-large");
  const nameEl = document.getElementById("profile-display-name");
  const emailEl = document.getElementById("profile-display-email");
  const phoneBadge = document.getElementById("profile-phone-badge");
  const inputName = document.getElementById("profile-input-name");
  const inputAvatar = document.getElementById("profile-input-avatar");
  const inputEmail = document.getElementById("profile-input-email");

  if (avatarEl) {
    if (buyer.avatar_url) {
      avatarEl.style.backgroundImage = `url('${escapeHtml(buyer.avatar_url)}')`;
      avatarEl.innerText = "";
    } else {
      avatarEl.style.backgroundImage = "none";
      avatarEl.innerText = (buyer.full_name || "P").charAt(0).toUpperCase();
    }
  }

  if (nameEl) nameEl.innerText = buyer.full_name || "Pembeli";
  if (emailEl) emailEl.innerText = buyer.email || "-";
  if (inputName) inputName.value = buyer.full_name || "";
  if (inputAvatar) inputAvatar.value = buyer.avatar_url || "";
  if (inputEmail) inputEmail.value = buyer.email || "";

  if (phoneBadge) {
    if (buyer.phone_verified && buyer.phone_number) {
      phoneBadge.innerHTML = `<span style="font-size:0.82rem; color:var(--emerald); font-weight:600">🟢 No. HP Terverifikasi: ${escapeHtml(buyer.phone_number)}</span>`;
    } else {
      phoneBadge.innerHTML = `
        <div style="display:inline-flex; align-items:center; gap:0.5rem">
          <span style="font-size:0.82rem; color:var(--rose); font-weight:600">🔴 Belum Verifikasi No. HP</span>
          <button type="button" class="btn-sm-action" onclick="openOtpModal()">Verifikasi</button>
        </div>
      `;
    }
  }

  // Fetch quick order statistics
  try {
    const res = await buyerAuthFetch("/api/v1/buyer/orders");
    if (res.ok) {
      const orders = await res.json();
      const totalOrdersEl = document.getElementById("stat-total-orders");
      const completedOrdersEl = document.getElementById("stat-completed-orders");
      if (totalOrdersEl) totalOrdersEl.innerText = orders.length;
      if (completedOrdersEl) {
        const completed = orders.filter(o => {
          const s = (o.status || "").toLowerCase();
          return s === "completed" || s === "delivered";
        }).length;
        completedOrdersEl.innerText = completed;
      }
    }
  } catch (e) {
    console.warn("Failed to load profile order stats:", e);
  }
}

async function handleUpdateBuyerProfile(event) {
  event.preventDefault();
  const inputName = document.getElementById("profile-input-name");
  const inputAvatar = document.getElementById("profile-input-avatar");
  const btnSave = document.getElementById("btn-save-profile");

  const full_name = inputName ? inputName.value.trim() : "";
  const avatar_url = inputAvatar && inputAvatar.value.trim() ? inputAvatar.value.trim() : null;

  if (full_name.length < 2) {
    showToast("Nama lengkap minimal 2 karakter.", "warning");
    return;
  }

  if (btnSave) {
    btnSave.disabled = true;
    btnSave.innerText = "Menyimpan...";
  }

  try {
    const res = await buyerAuthFetch("/api/v1/buyer/profile", {
      method: "PUT",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        full_name,
        avatar_url
      })
    });

    if (!res.ok) {
      const err = await res.json();
      showToast(`Gagal memperbarui profil: ${err.message || err.error || "Validasi gagal"}`, "error");
      return;
    }

    const updated = await res.json();
    if (window.StoreState) window.StoreState.activeBuyer = updated;
    window.activeBuyer = updated;
    localStorage.setItem("program1_buyer_user", JSON.stringify(updated));

    showToast("Profil Anda berhasil diperbarui!", "success");
    renderBuyerHeaderState();
    renderBuyerProfileDashboard();
  } catch (e) {
    console.error("handleUpdateBuyerProfile error:", e);
    showToast(`Terjadi kesalahan: ${e.message}`, "error");
  } finally {
    if (btnSave) {
      btnSave.disabled = false;
      btnSave.innerText = "💾 Simpan Perubahan Profil";
    }
  }
}

// Window Exports
window.buyerAuthFetch = buyerAuthFetch;
window.checkBuyerSession = checkBuyerSession;
window.syncBuyerProfile = syncBuyerProfile;
window.renderBuyerHeaderState = renderBuyerHeaderState;
window.openBuyerLoginModal = openBuyerLoginModal;
window.closeBuyerLoginModal = closeBuyerLoginModal;
window.fetchBuyerAuthConfig = fetchBuyerAuthConfig;
window.switchBuyerAuthTab = switchBuyerAuthTab;
window.handleBuyerLogin = handleBuyerLogin;
window.handleBuyerRegister = handleBuyerRegister;
window.devQuickBuyerLogin = devQuickBuyerLogin;
window.renderGoogleSignInButton = renderGoogleSignInButton;
window.handleGoogleCredentialResponse = handleGoogleCredentialResponse;
window.verifyGoogleIdToken = verifyGoogleIdToken;
window.logoutBuyer = logoutBuyer;
window.openOtpModal = openOtpModal;
window.closeBuyerOtpModal = closeBuyerOtpModal;
window.handleRequestOtp = handleRequestOtp;
window.startOtpCountdown = startOtpCountdown;
window.handleVerifyOtp = handleVerifyOtp;
window.handleResendOtp = handleResendOtp;
window.renderBuyerProfileDashboard = renderBuyerProfileDashboard;
window.handleUpdateBuyerProfile = handleUpdateBuyerProfile;

