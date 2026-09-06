/* ==========================================================================
   STOREFRONT HYBRID CHAT MODULE (WHATSAPP + IN-APP LIVE CHAT)
   File: /crates/web/static/store/store-chat.js
   ========================================================================== */

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
  const storeInfo = window.StoreState ? window.StoreState.storeInfo : window.storeInfo;
  const storeTitle = document.getElementById("chat-store-title");
  if (storeTitle && storeInfo) {
    storeTitle.innerText = `${storeInfo.store_name || "CS Toko"}`;
  }

  const memberBadge = document.getElementById("chat-member-badge");
  const memberName = document.getElementById("chat-member-name");
  const memberStatus = document.getElementById("chat-member-status");
  const fabLabel = document.getElementById("chat-fab-label");
  const fabBtn = document.getElementById("btn-floating-chat");

  const activeBuyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  const buyerToken = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;

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
  const activeBuyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  const buyerToken = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  const storeInfo = window.StoreState ? window.StoreState.storeInfo : window.storeInfo;
  const storeWhatsAppNumber = window.StoreState ? window.StoreState.storeWhatsAppNumber : window.storeWhatsAppNumber;

  // WhatsApp is ONLY accessible after registered or logged in
  if (!activeBuyer || !buyerToken) {
    toggleChatPopup(false);
    if (typeof openBuyerLoginModal === "function") openBuyerLoginModal();
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
  toggleChatPopup(false);
  const win = document.getElementById("inapp-chat-window");
  if (win) win.style.display = "flex";

  const inAppChatMessages = window.StoreState ? window.StoreState.inAppChatMessages : (window.inAppChatMessages || []);
  const storeInfo = window.StoreState ? window.StoreState.storeInfo : window.storeInfo;
  const activeBuyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  const buyerToken = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;

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

  const inAppChatMessages = window.StoreState ? window.StoreState.inAppChatMessages : (window.inAppChatMessages || []);

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
  const inAppChatMessages = window.StoreState ? window.StoreState.inAppChatMessages : (window.inAppChatMessages || []);
  const activeBuyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  const buyerToken = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;

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

// Window Exports
window.toggleChatPopup = toggleChatPopup;
window.updateFloatingChatWidget = updateFloatingChatWidget;
window.handleDirectWhatsAppChat = handleDirectWhatsAppChat;
window.openInAppChatWindow = openInAppChatWindow;
window.closeInAppChatWindow = closeInAppChatWindow;
window.renderInAppChatMessages = renderInAppChatMessages;
window.sendInAppChatMessage = sendInAppChatMessage;
