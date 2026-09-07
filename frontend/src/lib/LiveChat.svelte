<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { apiFetch } from './api';
  import { auth } from './auth.svelte';
  import type { ChatMessage } from './types';

  let isOpen = $state(false);
  let messages = $state<ChatMessage[]>([]);
  let inputText = $state('');
  let isSending = $state(false);
  let scrollContainer: HTMLDivElement;
  let intervalId: ReturnType<typeof setInterval>;

  async function fetchMessages() {
    if (!auth.user || (typeof document !== 'undefined' && document.hidden)) return;
    try {
      const data = await apiFetch<ChatMessage[]>('/v1/chat/messages');
      const hasNew = data.length > messages.length;
      messages = data;
      if (hasNew && isOpen) {
        await tick();
        scrollToBottom();
      }
    } catch {
      // Abaikan polling background error
    }
  }

  function scrollToBottom() {
    if (scrollContainer) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    }
  }

  async function sendMessage(e: Event) {
    e.preventDefault();
    if (!inputText.trim() || isSending) return;

    const content = inputText.trim();
    inputText = '';
    isSending = true;

    try {
      await apiFetch('/v1/chat/messages', {
        method: 'POST',
        body: JSON.stringify({ content }),
      });
      await fetchMessages();
    } finally {
      isSending = false;
    }
  }

  function toggleChat() {
    isOpen = !isOpen;
    if (isOpen) {
      fetchMessages();
      setTimeout(scrollToBottom, 100);
    }
  }

  onMount(() => {
    intervalId = setInterval(fetchMessages, 4000);
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });
</script>

<div class="chat-widget">
  {#if isOpen}
    <div class="chat-window">
      <div class="chat-head">
        <h4>💬 Live Chat Bantuan</h4>
        <button class="btn-close" onclick={toggleChat}>&times;</button>
      </div>

      <div class="chat-body" bind:this={scrollContainer}>
        {#if !auth.user}
          <p class="empty-hint">Silakan masuk / daftar untuk mengobrol dengan CS.</p>
        {:else if messages.length === 0}
          <p class="empty-hint">Mulai percakapan Anda dengan tim kami!</p>
        {:else}
          {#each messages as msg (msg.id)}
            <div class="msg-bubble {msg.sender_role.toLowerCase()}">
              <small class="sender">{msg.sender_name}</small>
              <p class="text">{msg.content}</p>
            </div>
          {/each}
        {/if}
      </div>

      {#if auth.user}
        <form onsubmit={sendMessage} class="chat-input-bar">
          <input
            type="text"
            placeholder="Tulis pesan..."
            bind:value={inputText}
            disabled={isSending}
          />
          <button type="submit" disabled={isSending || !inputText.trim()}>Kirim</button>
        </form>
      {/if}
    </div>
  {/if}

  <button class="chat-fab" onclick={toggleChat} aria-label="Buka Live Chat">
    💬
  </button>
</div>

<style>
  .chat-widget { position: fixed; bottom: 1.5rem; right: 1.5rem; z-index: 1000; }
  .chat-fab {
    width: 56px; height: 56px; border-radius: 50%; background: #0284c7;
    color: #fff; font-size: 1.5rem; border: none; cursor: pointer;
    box-shadow: 0 4px 16px rgba(0,0,0,0.5); transition: transform 0.2s, background 0.2s;
  }
  .chat-fab:hover { transform: scale(1.08); background: #0369a1; }
  .chat-window {
    position: absolute; bottom: 70px; right: 0; width: 340px; height: 440px;
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    display: flex; flex-direction: column; overflow: hidden;
    box-shadow: 0 8px 30px rgba(0,0,0,0.6);
  }
  .chat-head {
    background: #1e293b; padding: 0.8rem 1rem; border-bottom: 1px solid #334155;
    display: flex; justify-content: space-between; align-items: center; color: #fff;
  }
  .chat-head h4 { margin: 0; font-size: 0.95rem; }
  .btn-close { background: none; border: none; color: #94a3b8; font-size: 1.3rem; cursor: pointer; }
  .chat-body { flex: 1; padding: 1rem; overflow-y: auto; display: flex; flex-direction: column; gap: 0.6rem; }
  .empty-hint { text-align: center; color: #94a3b8; font-size: 0.85rem; margin-top: 3rem; }
  .msg-bubble { max-width: 80%; padding: 0.55rem 0.8rem; border-radius: 8px; font-size: 0.85rem; }
  .msg-bubble.buyer { align-self: flex-end; background: #0284c7; color: #fff; }
  .msg-bubble.seller, .msg-bubble.admin { align-self: flex-start; background: #1e293b; color: #f8fafc; border: 1px solid #334155; }
  .sender { font-size: 0.7rem; opacity: 0.8; display: block; margin-bottom: 0.2rem; }
  .text { margin: 0; word-break: break-word; }
  .chat-input-bar { display: flex; border-top: 1px solid #334155; padding: 0.5rem; background: #1e293b; }
  .chat-input-bar input {
    flex: 1; background: #0f172a; border: 1px solid #334155; border-radius: 4px;
    padding: 0.5rem; color: #fff;
  }
  .chat-input-bar button {
    background: #0284c7; color: white; border: none; padding: 0.5rem 0.8rem;
    border-radius: 4px; margin-left: 0.5rem; cursor: pointer; font-weight: bold;
  }
</style>
