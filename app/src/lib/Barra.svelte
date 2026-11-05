<script lang="ts">
  import { minimizar, fechar } from "./janela";
  import selo from "../assets/selo.png";
  let { versao }: { versao: string } = $props();
</script>

<div class="barra" data-tauri-drag-region>
  <div class="id" data-tauri-drag-region>
    <img class="selo" src={selo} alt="" width="18" height="18" draggable="false" />
    <span class="versao" data-tauri-drag-region>v{versao}</span>
  </div>
  <div class="ctl">
    <button type="button" aria-label="Minimizar" onclick={minimizar}>
      <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2 6.5h8" /></svg>
    </button>
    <button type="button" class="fechar" aria-label="Fechar" onclick={fechar}>
      <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" /></svg>
    </button>
  </div>
</div>

<style>
  /* A barra flutua sobre o app: sem fundo, sem linha. A cena sobe até o topo da janela. */
  .barra {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 5;
    height: 34px;
    display: flex;
    align-items: stretch;
    justify-content: space-between;
    user-select: none;
  }
  .id {
    display: flex;
    align-items: center;
    gap: 9px;
    padding-left: 14px;
    flex: 1;
  }
  .selo {
    width: 18px;
    height: 18px;
    display: block;
    pointer-events: none;
    filter: drop-shadow(0 1px 4px rgba(0, 0, 0, 0.5));
  }
  .versao {
    font-size: 11px;
    color: var(--mudo);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
  }
  .ctl {
    display: flex;
  }
  button {
    all: unset;
    box-sizing: border-box;
    width: 46px;
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.82);
    cursor: default;
    transition: background 0.12s, color 0.12s;
  }
  button svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: square;
  }
  button:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }
  button.fechar:hover {
    background: #b23a4b;
    color: #fff;
  }
  button:focus-visible {
    outline: 2px solid var(--ouro);
    outline-offset: -2px;
  }
</style>
