<script lang="ts">
  import type { Status } from "./tipos";
  let { status }: { status: Status | null } = $props();
</script>

<p class="estado" class:fora={!status?.online}>
  <i aria-hidden="true"></i>
  {#if status?.online}
    <span>Online</span><span class="n">{status.jogadores}/{status.max}</span>
  {:else}
    <span>Sem conexão</span>
  {/if}
</p>

<style>
  .estado {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--mudo);
  }
  i {
    width: 8px;
    height: 8px;
    background: #6fd49a;
    box-shadow: 0 0 0 3px rgba(111, 212, 154, 0.18);
  }
  .fora i {
    background: #e07888;
    box-shadow: 0 0 0 3px rgba(224, 120, 136, 0.18);
  }
  .n {
    font-variant-numeric: tabular-nums;
    color: var(--texto);
  }
</style>
