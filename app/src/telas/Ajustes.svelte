<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { expoOut } from "svelte/easing";
  import type { Ajustes, AjustesEditaveis } from "../lib/tipos";
  import Falha from "../lib/Falha.svelte";
  import Andamento from "../lib/Andamento.svelte";
  import type { Progresso } from "../lib/tipos";
  let {
    dados,
    salvar,
    mudar,
    verificar,
    abrir,
    voltar,
    ocupado,
    progresso = null,
    erro = null,
    tentar,
  }: {
    dados: Ajustes;
    salvar: (a: AjustesEditaveis) => void;
    mudar: () => void;
    verificar: () => void;
    abrir: () => void;
    voltar?: () => void;
    ocupado: boolean;
    progresso?: Progresso | null;
    erro?: string | null;
    tentar?: () => void;
  } = $props();
  let gb = $derived(dados.memoria_efetiva / 1024);
  const gbTxt = (n: number) => (n / 1024 ** 3).toLocaleString("pt-BR", { maximumFractionDigits: 1 });
</script>

<div class="tela" in:fly={{ x: 28, duration: 520, easing: expoOut }} out:fade={{ duration: 140 }}>
  <header>
    <h1>Ajustes</h1>
  </header>

  <fieldset disabled={ocupado}>
    <div class="campo">
      <div class="linha">
        <label for="ram">Memória</label>
        <output for="ram">{gb.toLocaleString("pt-BR")} GB</output>
      </div>
      <input
        id="ram"
        type="range"
        min="2048"
        max={dados.memoria_maxima}
        step="256"
        value={Math.min(dados.memoria_efetiva, dados.memoria_maxima)}
        onchange={(e) =>
          salvar({ memoria_mb: Number(e.currentTarget.value), shader: dados.shader, visual: dados.visual })}
      />
      {#if dados.memoria_total < 8192}<p class="aviso">Menos de 8 GB de RAM</p>{/if}
      <button
        type="button"
        class="link-mudo"
        onclick={() => salvar({ memoria_mb: null, shader: dados.shader, visual: dados.visual })}>Restaurar padrão</button
      >
    </div>

    <div class="campo">
      <div class="linha">
        <span class="rotulo">Pasta do jogo</span>
        <button type="button" class="vidro-btn" onclick={mudar}>Mudar pasta</button>
      </div>
      <p class="caminho">{dados.pasta}</p>
    </div>

    <div class="linha">
      <label for="visual" class="duplo"
        >Pacote visual<small>Mods visuais (Voxy, partículas, animações) · {gbTxt(dados.visual_bytes)} GB</small></label
      >
      <input
        id="visual"
        class="chave"
        type="checkbox"
        checked={dados.visual}
        onchange={(e) =>
          salvar({ memoria_mb: dados.memoria_mb, shader: dados.shader, visual: e.currentTarget.checked })}
      />
    </div>

    <div class="linha">
      <label for="shader">Shader</label>
      <input
        id="shader"
        class="chave"
        type="checkbox"
        checked={dados.shader}
        onchange={(e) =>
          salvar({ memoria_mb: dados.memoria_mb, shader: e.currentTarget.checked, visual: dados.visual })}
      />
    </div>

    <div class="acoes">
      <button type="button" class="vidro-btn" onclick={verificar}>Verificar arquivos</button>
      <button type="button" class="vidro-btn" onclick={abrir}>Abrir pasta</button>
    </div>
  </fieldset>

  <div class="base">
    <Andamento {progresso} {ocupado} padrao="Verificando arquivos…" />
    <Falha {erro} {tentar} />
    {#if voltar}<button type="button" class="acao" onclick={voltar}>Voltar</button>{/if}
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
  }
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
    min-width: 0;
    display: grid;
    gap: 18px;
  }
  fieldset:disabled {
    opacity: 0.6;
  }
  .campo {
    display: grid;
    gap: 9px;
  }
  .linha {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  label,
  .rotulo {
    font-size: 13.5px;
    font-weight: 650;
  }
  output {
    font-family: var(--pixel);
    font-size: 20px;
    font-weight: 700;
    color: var(--ouro);
  }
  input[type="range"] {
    width: 100%;
    accent-color: var(--ouro);
    height: 20px;
    margin: 0;
    cursor: pointer;
  }
  input[type="range"]:focus-visible,
  .chave:focus-visible {
    outline: 2px solid var(--ouro);
    outline-offset: 3px;
  }
  .aviso {
    margin: 0;
    font-size: 12.5px;
    color: #ffb4bd;
  }
  .caminho {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--mudo);
    overflow-wrap: anywhere;
  }
  .duplo {
    display: grid;
    gap: 2px;
  }
  .duplo small {
    font-size: 12px;
    font-weight: 500;
    color: var(--mudo);
  }
  .acoes {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .base {
    display: grid;
    gap: 12px;
  }
</style>
