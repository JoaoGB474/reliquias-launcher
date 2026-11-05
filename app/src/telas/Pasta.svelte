<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { expoOut } from "svelte/easing";
  import type { PastaInfo } from "../lib/tipos";
  import { mensagemErro } from "../lib/ponte";
  let {
    dados,
    mudando,
    ocupado,
    escolher,
    confirmar,
    abrir,
    voltar,
  }: {
    dados: PastaInfo;
    mudando: boolean;
    ocupado: boolean;
    escolher: () => void;
    confirmar: (mover: boolean, visual: boolean) => void;
    abrir: () => void;
    voltar?: () => void;
  } = $props();
  const gb = (n: number) =>
    (n / 1024 ** 3).toLocaleString("pt-BR", { maximumFractionDigits: 1 });
  let bloqueado = $derived(ocupado || !!dados.erro);
  let escolhido = $state<boolean | null>(null);
  let visual = $derived(escolhido ?? dados.visual);
</script>

<div class="tela" in:fly={{ x: 28, duration: 520, easing: expoOut }} out:fade={{ duration: 140 }}>
  <h1>Pasta do jogo</h1>

  <div class="meio">
    <p class="caminho">{dados.caminho}</p>
    <p class="espaco">{gb(dados.livre)} GB livres · {gb(dados.estimado)} GB estimados</p>
    {#if dados.erro || dados.aviso}
      <p class="aviso" class:erro={!!dados.erro} role="alert">
        {mensagemErro({ codigo: dados.erro || dados.aviso })}
      </p>
    {/if}
    <label class="opcao">
      <input
        class="chave"
        type="checkbox"
        checked={visual}
        disabled={ocupado}
        onchange={(e) => (escolhido = e.currentTarget.checked)}
      />
      <span class="texto"
        >Pacote visual<small>Mods visuais: Voxy, partículas e animações · {gb(dados.visual_bytes)} GB</small></span
      >
    </label>
    <div class="acoes">
      <button type="button" class="vidro-btn" disabled={ocupado} onclick={escolher}>Escolher outra</button>
      <button type="button" class="vidro-btn" disabled={ocupado} onclick={abrir}>Abrir pasta</button>
    </div>
  </div>

  <div class="base">
    {#if mudando}
      <button type="button" class="acao" disabled={bloqueado} onclick={() => confirmar(true, visual)}>Mover os arquivos</button>
      <button type="button" class="vidro-btn" disabled={bloqueado} onclick={() => confirmar(false, visual)}>Instalar de novo</button>
    {:else}
      <button type="button" class="acao" disabled={bloqueado} onclick={() => confirmar(false, visual)}>Instalar</button>
    {/if}
    {#if voltar && mudando}<button type="button" class="link-mudo" onclick={voltar}>Voltar</button>{/if}
  </div>
</div>

<style>
  .tela {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 16px;
    min-height: 0;
  }
  h1 {
    margin: 4px 0 0;
    font-family: var(--pixel);
    font-size: 34px;
    font-weight: 700;
    line-height: 1;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    max-width: 8ch;
  }
  .meio {
    display: grid;
    gap: 10px;
  }
  p {
    margin: 0;
  }
  .caminho {
    padding: 12px 14px;
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.09);
    font-size: 13px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .espaco {
    font-size: 12.5px;
    color: var(--mudo);
    font-variant-numeric: tabular-nums;
  }
  .aviso {
    font-size: 13px;
    line-height: 1.35;
    color: #f3d9a0;
  }
  .aviso.erro {
    color: #ffb4bd;
  }
  .opcao {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 4px;
    cursor: pointer;
  }
  .texto {
    display: grid;
    gap: 2px;
    font-size: 13.5px;
    font-weight: 650;
  }
  .texto small {
    font-size: 12px;
    font-weight: 500;
    color: var(--mudo);
  }
  .acoes {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 4px;
  }
  .base {
    display: grid;
    gap: 12px;
  }
</style>
