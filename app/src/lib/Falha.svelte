<script lang="ts">
  let { erro, tentar }: { erro: string | null | undefined; tentar?: () => void } = $props();
</script>

{#if erro}
  <p class="falha" role="alert">
    <span>{erro}</span>
    {#if tentar}<button type="button" onclick={tentar}>Tentar de novo</button>{/if}
  </p>
{/if}

<style>
  .falha {
    margin: 0;
    padding: 10px 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 13px;
    line-height: 1.35;
    color: #ffc2c8;
    background: rgba(138, 44, 58, 0.28);
    border: 1px solid rgba(214, 96, 112, 0.45);
    border-radius: 8px;
  }
  button {
    all: unset;
    flex: none;
    font-size: 13px;
    font-weight: 700;
    color: #fff;
    cursor: pointer;
    border-bottom: 1px solid rgba(255, 255, 255, 0.5);
  }
  button:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 3px;
  }
</style>
