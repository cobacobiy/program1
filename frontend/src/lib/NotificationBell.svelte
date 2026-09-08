<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { NotificationItem, NotificationCount } from './types';
  import { apiFetch } from './api';
  import { adminAuth } from './adminAuth.svelte';
  import { auth } from './auth.svelte';
  import { toast } from './toast.svelte';

  interface Props {
    role?: 'buyer' | 'admin';
    onNotificationClick?: (item: NotificationItem) => void;
  }

  let { role = 'buyer', onNotificationClick }: Props = $props();

  let isOpen = $state(false);
  let unreadCount = $state(0);
  let notifications = $state<NotificationItem[]>([]);
  let loading = $state(false);
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  function timeAgo(dateString: string): string {
    const diff = Date.now() - new Date(dateString).getTime();
    const minutes = Math.floor(diff / 60000);
    if (minutes < 1) return 'Baru saja';
    if (minutes < 60) return `${minutes} mnt lalu`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours} jam lalu`;
    const days = Math.floor(hours / 24);
    return `${days} hari lalu`;
  }

  function getTypeIcon(type: string): string {
    switch (type) {
      case 'order_status':
        return '📦';
      case 'new_order':
        return '🛒';
      case 'low_stock':
        return '⚠️';
      case 'chat':
        return '💬';
      case 'promo':
        return '🏷️';
      default:
        return '🔔';
    }
  }

  async function fetchCounts() {
    if (role === 'buyer' && !auth.user) {
      unreadCount = 0;
      return;
    }
    if (role === 'admin' && !adminAuth.token) {
      unreadCount = 0;
      return;
    }

    try {
      if (role === 'buyer') {
        const res = await apiFetch<NotificationCount>('/api/v1/buyer/notifications/count');
        unreadCount = res.unread;
      } else {
        const res = await fetch('/api/v1/admin/notifications/count', {
          headers: { Authorization: `Bearer ${adminAuth.token}` },
        });
        if (res.ok) {
          const data: NotificationCount = await res.json();
          unreadCount = data.unread;
        }
      }
    } catch {
      // silent fallback
    }
  }

  async function fetchNotifications() {
    loading = true;
    try {
      if (role === 'buyer') {
        notifications = await apiFetch<NotificationItem[]>('/api/v1/buyer/notifications?limit=20');
      } else {
        const res = await fetch('/api/v1/admin/notifications?limit=20', {
          headers: { Authorization: `Bearer ${adminAuth.token}` },
        });
        if (res.ok) {
          notifications = await res.json();
        }
      }
    } catch {
      notifications = [];
    } finally {
      loading = false;
    }
  }

  async function toggleDropdown() {
    isOpen = !isOpen;
    if (isOpen) {
      await fetchNotifications();
      await fetchCounts();
    }
  }

  async function handleMarkRead(item: NotificationItem, e: MouseEvent) {
    e.stopPropagation();
    if (item.is_read) {
      if (onNotificationClick) onNotificationClick(item);
      isOpen = false;
      return;
    }

    try {
      if (role === 'buyer') {
        await apiFetch(`/api/v1/buyer/notifications/${item.id}/read`, { method: 'POST' });
      } else {
        await fetch(`/api/v1/admin/notifications/${item.id}/read`, {
          method: 'POST',
          headers: { Authorization: `Bearer ${adminAuth.token}` },
        });
      }
      item.is_read = true;
      unreadCount = Math.max(0, unreadCount - 1);
    } catch {
      // silent
    }

    if (onNotificationClick) onNotificationClick(item);
    isOpen = false;
  }

  async function handleMarkAllRead() {
    try {
      if (role === 'buyer') {
        await apiFetch('/api/v1/buyer/notifications/read-all', { method: 'POST' });
      } else {
        await fetch('/api/v1/admin/notifications/read-all', {
          method: 'POST',
          headers: { Authorization: `Bearer ${adminAuth.token}` },
        });
      }
      notifications = notifications.map((n) => ({ ...n, is_read: true }));
      unreadCount = 0;
      toast.success('Semua notifikasi telah ditandai dibaca.');
    } catch (err: any) {
      toast.error(err.message || 'Gagal menandai semua dibaca.');
    }
  }

  onMount(() => {
    fetchCounts();
    pollTimer = setInterval(fetchCounts, 30000);
  });

  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
  });
</script>

<div class="notif-container">
  <button
    class="btn-bell"
    onclick={toggleDropdown}
    type="button"
    aria-label="Notifikasi"
    title="Pusat Notifikasi"
  >
    <span class="bell-icon">🔔</span>
    {#if unreadCount > 0}
      <span class="badge-count">{unreadCount > 99 ? '99+' : unreadCount}</span>
    {/if}
  </button>

  {#if isOpen}
    <div
      class="backdrop-dismiss"
      onclick={() => (isOpen = false)}
      role="presentation"
      tabindex="-1"
    ></div>

    <div class="notif-dropdown" role="dialog" aria-modal="true">
      <div class="dropdown-header">
        <div class="header-title">
          <h4>Notifikasi</h4>
          {#if unreadCount > 0}
            <span class="unread-pill">{unreadCount} baru</span>
          {/if}
        </div>
        {#if unreadCount > 0}
          <button class="btn-read-all" onclick={handleMarkAllRead} type="button">
            Tandai Dibaca
          </button>
        {/if}
      </div>

      <div class="dropdown-body">
        {#if loading}
          <div class="notif-state-box">
            <p>Memuat notifikasi...</p>
          </div>
        {:else if notifications.length === 0}
          <div class="notif-state-box empty">
            <span class="empty-icon">🔕</span>
            <p>Belum ada notifikasi saat ini.</p>
          </div>
        {:else}
          <div class="notif-list">
            {#each notifications as item (item.id)}
              <div
                class="notif-card"
                class:unread={!item.is_read}
                onclick={(e) => handleMarkRead(item, e)}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === 'Enter' && handleMarkRead(item, e as any)}
              >
                <div class="card-icon">{getTypeIcon(item.notification_type)}</div>
                <div class="card-content">
                  <div class="content-top">
                    <strong class="item-title">{item.title}</strong>
                    {#if !item.is_read}
                      <span class="dot-unread"></span>
                    {/if}
                  </div>
                  <p class="item-msg">{item.message}</p>
                  <small class="item-time">{timeAgo(item.created_at)}</small>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .notif-container {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .btn-bell {
    position: relative;
    background: #1e293b;
    border: 1px solid #334155;
    color: #cbd5e1;
    padding: 0.45rem 0.65rem;
    border-radius: 8px;
    cursor: pointer;
    font-size: 1.1rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .btn-bell:hover {
    background: #334155;
    color: #fff;
    border-color: #475569;
  }

  .badge-count {
    position: absolute;
    top: -6px;
    right: -6px;
    background: #ef4444;
    color: #ffffff;
    font-size: 0.65rem;
    font-weight: 800;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    border-radius: 999px;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 5px rgba(239, 68, 68, 0.4);
    animation: pulseBadge 2s infinite;
  }

  @keyframes pulseBadge {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.1); }
  }

  .backdrop-dismiss {
    position: fixed;
    inset: 0;
    z-index: 1000;
    cursor: default;
  }

  .notif-dropdown {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    width: 340px;
    max-height: 460px;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 12px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
    z-index: 1001;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: slideDown 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideDown {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .dropdown-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.85rem 1rem;
    background: #1e293b;
    border-bottom: 1px solid #334155;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .header-title h4 {
    margin: 0;
    color: #f8fafc;
    font-size: 0.95rem;
  }

  .unread-pill {
    background: #0284c7;
    color: #fff;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 0.15rem 0.45rem;
    border-radius: 10px;
  }

  .btn-read-all {
    background: transparent;
    border: none;
    color: #38bdf8;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    transition: background 0.15s;
  }

  .btn-read-all:hover {
    background: rgba(56, 189, 248, 0.15);
  }

  .dropdown-body {
    flex: 1;
    overflow-y: auto;
    max-height: 400px;
  }

  .notif-state-box {
    padding: 2.5rem 1rem;
    text-align: center;
    color: #94a3b8;
    font-size: 0.85rem;
  }

  .notif-state-box.empty .empty-icon {
    font-size: 2rem;
    display: block;
    margin-bottom: 0.5rem;
    opacity: 0.7;
  }

  .notif-list {
    display: flex;
    flex-direction: column;
  }

  .notif-card {
    display: flex;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #1e293b;
    cursor: pointer;
    background: transparent;
    transition: background 0.15s ease;
    text-align: left;
  }

  .notif-card:last-child {
    border-bottom: none;
  }

  .notif-card:hover {
    background: #1e293b;
  }

  .notif-card.unread {
    background: rgba(2, 132, 199, 0.08);
  }

  .card-icon {
    font-size: 1.25rem;
    line-height: 1;
    margin-top: 2px;
  }

  .card-content {
    flex: 1;
    min-width: 0;
  }

  .content-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .item-title {
    font-size: 0.85rem;
    color: #f1f5f9;
    line-height: 1.25;
  }

  .dot-unread {
    width: 7px;
    height: 7px;
    background: #38bdf8;
    border-radius: 50%;
    margin-top: 4px;
    flex-shrink: 0;
  }

  .item-msg {
    margin: 0.25rem 0 0.35rem;
    font-size: 0.75rem;
    color: #94a3b8;
    line-height: 1.4;
    word-break: break-word;
  }

  .item-time {
    font-size: 0.7rem;
    color: #64748b;
  }
</style>
