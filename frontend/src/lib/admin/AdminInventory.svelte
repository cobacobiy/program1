<script lang="ts">
  import type { Product } from '../types';
  import './admin-shared.css';

  interface Props {
    products: Product[];
  }

  let { products }: Props = $props();
</script>

<div class="section-panel">
  <h2>🏭 Monitoring Stok & Inventori</h2>
  <div style="overflow-x: auto;">
    <table class="table-custom">
      <thead>
        <tr>
          <th>Nama Barang</th>
          <th>Stok Tersedia</th>
          <th>Kondisi</th>
        </tr>
      </thead>
      <tbody>
        {#each products as p (p.id)}
          <tr>
            <td><strong>{p.name}</strong></td>
            <td>{p.stock} unit</td>
            <td>
              {#if p.stock <= 0}
                <span class="badge-crit">Habis</span>
              {:else if p.stock <= 5}
                <span class="badge-warn">Kritis (&le;5)</span>
              {:else}
                <span class="badge-safe">Aman</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .badge-crit { background: #dc2626; color: #fff; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
  .badge-warn { background: #d97706; color: #fff; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
  .badge-safe { background: #059669; color: #fff; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
</style>
