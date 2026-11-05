<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { expoOut } from "svelte/easing";
  import type { Status } from "../lib/tipos";
  import Marca from "../lib/Marca.svelte";
  import StatusServidor from "../lib/StatusServidor.svelte";
  import Falha from "../lib/Falha.svelte";
  let {
    ocupado,
    entrar,
    cancelar,
    status = null,
    erro = null,
    tentar,
  }: {
    ocupado: boolean;
    entrar: () => void;
    cancelar: () => void;
    status?: Status | null;
    erro?: string | null;
    tentar?: () => void;
  } = $props();
</script>

<div class="tela" in:fly={{ x: -28, duration: 520, easing: expoOut }} out:fade={{ duration: 140 }}>
  <Marca />
  <div class="base">
    <Falha {erro} {tentar} />
    {#if ocupado}
      <p class="espera" role="status">Aguardando o navegador…</p>
      <div class="trilha" aria-hidden="true"><i></i></div>
      <button type="button" class="link-mudo" onclick={cancelar}>Cancelar</button>
    {:else}
      <button type="button" class="acao" onclick={entrar}>
        <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true"
          ><path
            d="M20.317 4.3698a19.7913 19.7913 0 00-4.8851-1.5152.0741.0741 0 00-.0785.0371c-.211.3753-.4447.8648-.6083 1.2495-1.8447-.2762-3.68-.2762-5.4868 0-.1636-.3933-.4058-.8742-.6177-1.2495a.077.077 0 00-.0785-.037 19.7363 19.7363 0 00-4.8852 1.515.0699.0699 0 00-.0321.0277C.5334 9.0458-.319 13.5799.0992 18.0578a.0824.0824 0 00.0312.0561c2.0528 1.5076 4.0413 2.4228 5.9929 3.0294a.0777.0777 0 00.0842-.0276c.4616-.6304.8731-1.2952 1.226-1.9942a.076.076 0 00-.0416-.1057c-.6528-.2476-1.2743-.5495-1.8722-.8923a.077.077 0 01-.0076-.1277c.1258-.0943.2517-.1923.3718-.2914a.0743.0743 0 01.0776-.0105c3.9278 1.7933 8.18 1.7933 12.0614 0a.0739.0739 0 01.0785.0095c.1202.099.246.1981.3728.2924a.077.077 0 01-.0066.1276 12.2986 12.2986 0 01-1.873.8914.0766.0766 0 00-.0407.1067c.3604.698.7719 1.3628 1.225 1.9932a.076.076 0 00.0842.0286c1.961-.6067 3.9495-1.5219 6.0023-3.0294a.077.077 0 00.0313-.0552c.5004-5.177-.8382-9.6739-3.5485-13.6604a.061.061 0 00-.0312-.0286zM8.02 15.3312c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9555-2.4189 2.157-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.9555 2.4189-2.1569 2.4189zm7.9748 0c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9554-2.4189 2.1569-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.946 2.4189-2.1568 2.4189Z"
          /></svg
        >
        <span>Entrar com Discord</span>
      </button>
    {/if}
    <StatusServidor {status} />
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
  .espera {
    margin: 0;
    font-family: var(--pixel);
    font-size: 17px;
    font-weight: 600;
    letter-spacing: 0.03em;
    color: var(--texto);
  }
  .trilha {
    height: 3px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.09);
    overflow: hidden;
  }
  .trilha i {
    display: block;
    width: 38%;
    height: 100%;
    background: linear-gradient(90deg, var(--ouro), #fff0c8);
    border-radius: 2px;
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
