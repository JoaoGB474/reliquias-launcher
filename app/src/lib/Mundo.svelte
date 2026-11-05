<script lang="ts">
  import Aurora from "./Aurora.svelte";
  import { dataCurta, type Novidades } from "./novidades";
  let {
    novidades,
    aberto = false,
    offline = false,
  }: { novidades: Novidades; aberto?: boolean; offline?: boolean } = $props();
  type Aba = "servidor" | "modpack";
  let aba = $state<Aba>("servidor");
  let lista = $derived(novidades[aba]);
  let escolhida = $state<number | null>(null);
  let aberta = $derived(escolhida ?? lista[0]?.versao ?? null);
  function trocar(a: Aba) {
    aba = a;
    escolhida = null;
  }
</script>

<div class="palco" class:aberto>
  <Aurora {aberto} />
  <div class="grao" aria-hidden="true"></div>

  <section class="vidro" aria-label="Novidades">
    <div class="abas" role="tablist">
      <button
        type="button"
        role="tab"
        class="aba ouro"
        aria-selected={aba === "servidor"}
        onclick={() => trocar("servidor")}>Servidor</button
      >
      <button
        type="button"
        role="tab"
        class="aba ametista"
        aria-selected={aba === "modpack"}
        onclick={() => trocar("modpack")}>Modpack</button
      >
    </div>
    <div class="lista">
      {#each lista as n (n.versao)}
        {@const ativa = aberta === n.versao}
        <article class:ativa>
          <button
            type="button"
            class="item"
            aria-expanded={ativa}
            onclick={() => (escolhida = ativa ? -1 : n.versao)}
          >
            <span class="rv">v{n.versao}</span>
            <span class="rt">{n.titulo}</span>
            <span class="rd">{dataCurta(n.data)}</span>
          </button>
          {#if ativa}
            <ul>
              {#each n.itens as item}<li>{item}</li>{/each}
            </ul>
          {/if}
        </article>
      {:else}
        <p class="vazio">{offline ? "Servidor fora do ar" : "Sem novidades ainda"}</p>
      {/each}
    </div>
  </section>
</div>

<style>
  .palco {
    position: relative;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    background: #0a0713;
    display: flex;
    justify-content: flex-end;
    align-items: stretch;
    padding: 50px 28px 26px;
  }
  .palco::before {
    content: "";
    position: absolute;
    inset: 0 0 auto 0;
    height: 70px;
    z-index: 1;
    pointer-events: none;
    background: linear-gradient(to bottom, rgba(8, 5, 16, 0.62), rgba(8, 5, 16, 0));
  }
  .grao {
    position: absolute;
    inset: 0;
    opacity: 0.07;
    mix-blend-mode: overlay;
    pointer-events: none;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='160' height='160'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='2' stitchTiles='stitch'/></filter><rect width='100%' height='100%' filter='url(%23n)'/></svg>");
  }
  .vidro {
    position: relative;
    width: min(100%, 440px);
    align-self: flex-start;
    max-height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-radius: 14px;
    background: rgba(16, 11, 30, 0.58);
    backdrop-filter: blur(22px) saturate(1.35);
    border: 1px solid rgba(255, 255, 255, 0.09);
    box-shadow:
      0 28px 56px -18px rgba(0, 0, 0, 0.7),
      inset 0 1px 0 rgba(255, 255, 255, 0.07);
    overflow: hidden;
  }
  .abas {
    display: flex;
    gap: 6px;
    padding: 12px 12px 0;
  }
  .aba {
    all: unset;
    box-sizing: border-box;
    flex: 1;
    text-align: center;
    padding: 10px 0 11px;
    font-family: var(--pixel);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--mudo);
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: color 0.18s, border-color 0.18s;
  }
  .aba:hover {
    color: var(--texto);
  }
  .aba[aria-selected="true"] {
    color: var(--texto);
  }
  .aba.ouro[aria-selected="true"] {
    border-color: var(--ouro);
  }
  .aba.ametista[aria-selected="true"] {
    border-color: var(--ametista);
  }
  .aba:focus-visible {
    outline: 2px solid var(--ouro);
    outline-offset: -2px;
    border-radius: 6px;
  }
  .lista {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px 12px 14px;
  }
  article {
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }
  article:last-child {
    border-bottom: 0;
  }
  .item {
    all: unset;
    box-sizing: border-box;
    width: 100%;
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: baseline;
    gap: 12px;
    padding: 14px 4px;
    cursor: pointer;
    color: var(--mudo);
    transition: color 0.18s;
  }
  .item:hover,
  .ativa .item {
    color: var(--texto);
  }
  .item:focus-visible {
    outline: 2px solid var(--ouro);
    outline-offset: 0;
    border-radius: 6px;
  }
  .rv {
    font-family: var(--pixel);
    font-size: 22px;
    font-weight: 700;
    color: var(--ouro);
    min-width: 2.2ch;
  }
  .rt {
    font-size: 14px;
    font-weight: 600;
    line-height: 1.3;
  }
  .rd {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--mudo);
  }
  ul {
    margin: 0 4px 14px 46px;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 7px;
  }
  li {
    position: relative;
    padding-left: 14px;
    font-size: 13px;
    line-height: 1.45;
    color: #cfc6e4;
    max-width: 62ch;
  }
  li::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0.5em;
    width: 5px;
    height: 5px;
    background: var(--ametista);
  }
  .vazio {
    padding: 24px 4px;
    color: var(--mudo);
    font-size: 13px;
  }
</style>
