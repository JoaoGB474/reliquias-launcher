<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { expoOut } from "svelte/easing";
  import type { Progresso, Status } from "../lib/tipos";
  import Marca from "../lib/Marca.svelte";
  import StatusServidor from "../lib/StatusServidor.svelte";
  import Falha from "../lib/Falha.svelte";
  import Andamento from "../lib/Andamento.svelte";
  let {
    ocupado,
    progresso,
    status,
    atualizacao,
    jogar,
    ajustes,
    sair,
    erro = null,
    tentar,
  }: {
    ocupado: boolean;
    progresso: Progresso | null;
    status: Status | null;
    atualizacao: boolean;
    jogar: () => void;
    ajustes?: () => void;
    sair?: () => void;
    erro?: string | null;
    tentar?: () => void;
  } = $props();
</script>

<div class="tela" in:fly={{ x: 28, duration: 520, easing: expoOut }} out:fade={{ duration: 140 }}>
  <Marca />
  <div class="base">
    <Falha {erro} {tentar} />
    {#if atualizacao}
      <p class="faixa" role="status"><i aria-hidden="true"></i>Atualização pronta</p>
    {/if}
    <Andamento {progresso} {ocupado} />
    <button type="button" class="acao" disabled={ocupado} onclick={jogar}>Jogar</button>
    <StatusServidor {status} />
    {#if ajustes || sair}
      <nav aria-label="Principal">
        {#if ajustes}<button type="button" class="link-mudo" onclick={ajustes}>Ajustes</button>{/if}
        {#if sair}<button type="button" class="link-mudo" disabled={ocupado} onclick={sair}>Sair</button>{/if}
      </nav>
    {/if}
  </div>
</div>

<style>
  .tela {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    min-height: 0;
  }
  .base {
    display: grid;
    gap: 14px;
  }
  .faixa {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 13px;
    font-weight: 650;
    color: #d8c6fa;
  }
  .faixa i {
    width: 7px;
    height: 7px;
    background: var(--ametista);
    box-shadow: 0 0 0 3px rgba(157, 114, 230, 0.25);
  }
  nav {
    display: flex;
    gap: 18px;
  }
</style>
