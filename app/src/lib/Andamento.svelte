<script lang="ts">
  import type { Progresso } from "./tipos";
  let {
    progresso,
    ocupado = false,
    padrao = "Preparando…",
  }: { progresso: Progresso | null; ocupado?: boolean; padrao?: string } = $props();
  let medido = $derived(!!progresso && progresso.total > 0);
  let fracao = $derived(medido && progresso ? Math.min(1, progresso.feito / progresso.total) : 0);
  let rotulo = $derived(
    medido && progresso
      ? `${progresso.etapa} ${progresso.feito}/${progresso.total}`
      : (progresso?.etapa ?? padrao),
  );
</script>

{#if progresso || ocupado}
  <div class="andamento" role="status">
    <div class="topo">
      <span class="rotulo">{rotulo}</span>
      {#if medido}<span class="pct">{Math.round(fracao * 100)}%</span>{/if}
    </div>
    <div
      class="trilha"
      role="progressbar"
      aria-label={rotulo}
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={medido ? Math.round(fracao * 100) : undefined}
    >
      {#if medido}
        <i class="cheia" style:transform={`scaleX(${fracao})`}></i>
      {:else}
        <i class="vaga"></i>
      {/if}
    </div>
  </div>
{/if}

<style>
  .andamento {
    display: grid;
    gap: 8px;
  }
  .topo {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  .rotulo {
    font-family: var(--pixel);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: 0.03em;
  }
  .pct {
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    color: var(--mudo);
  }
  .trilha {
    height: 6px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.09);
    overflow: hidden;
  }
  i {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: linear-gradient(90deg, var(--ouro), #fff0c8);
  }
  .cheia {
    width: 100%;
    transform-origin: left;
    transition: transform 0.4s var(--expo);
  }
  .vaga {
    width: 38%;
    animation: varre 1.5s cubic-bezier(0.65, 0, 0.35, 1) infinite;
  }
  @keyframes varre {
    from {
      transform: translateX(-110%);
    }
    to {
      transform: translateX(290%);
    }
  }
</style>
