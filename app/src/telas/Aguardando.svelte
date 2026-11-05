<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { expoOut } from "svelte/easing";
  let {
    id,
    copiar,
    entrar,
  }: { id: string; copiar: () => void; entrar: () => void } = $props();
</script>

<div class="aguardando" in:fly={{ x: 28, duration: 520, easing: expoOut }} out:fade={{ duration: 140 }}>
  <h1>Aguardando aprovação</h1>
  <div class="base">
    <div class="id">
      <code>{id}</code>
      <button type="button" class="copiar" onclick={copiar}>Copiar</button>
    </div>
    <button type="button" class="outra" onclick={entrar}>Tentar de novo</button>
  </div>
</div>

<style>
  .aguardando {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
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
    color: var(--texto);
    max-width: 8ch;
  }
  .base {
    display: grid;
    gap: 14px;
  }
  .id {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 12px 12px 12px 14px;
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.09);
  }
  code {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 14px;
    letter-spacing: 0.02em;
    color: var(--texto);
    overflow-wrap: anywhere;
    user-select: all;
  }
  .copiar {
    all: unset;
    flex: none;
    padding: 7px 12px;
    border-radius: 7px;
    font-size: 13px;
    font-weight: 700;
    color: #1b1305;
    background: var(--ouro);
    cursor: pointer;
    transition: filter 0.15s;
  }
  .copiar:hover {
    filter: brightness(1.08);
  }
  .copiar:focus-visible,
  .outra:focus-visible {
    outline: 2px solid #fff3d2;
    outline-offset: 3px;
  }
  .outra {
    all: unset;
    justify-self: start;
    font-size: 13px;
    font-weight: 600;
    color: var(--mudo);
    cursor: pointer;
    padding: 6px 2px;
    border-bottom: 1px solid rgba(168, 159, 140, 0.35);
    transition: color 0.15s, border-color 0.15s;
  }
  .outra:hover {
    color: var(--texto);
    border-color: var(--texto);
  }
</style>
