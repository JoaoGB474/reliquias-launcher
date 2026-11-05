<script lang="ts">
  import type { Snippet } from "svelte";
  import Mundo from "./Mundo.svelte";
  import type { Novidades } from "./novidades";
  let {
    novidades,
    aberto = false,
    offline = false,
    children,
  }: { novidades: Novidades; aberto?: boolean; offline?: boolean; children: Snippet } = $props();
</script>

<div class="cena" class:aberto>
  <section class="painel">{@render children()}</section>
  <div class="fenda" aria-hidden="true"></div>
  <Mundo {novidades} {aberto} {offline} />
</div>

<style>
  .cena {
    flex: 1;
    min-height: 0;
    display: flex;
    position: relative;
  }
  .painel {
    position: relative;
    width: 340px;
    flex: none;
    display: grid;
    padding: 58px 30px 26px;
    background:
      radial-gradient(120% 55% at 0% 100%, rgba(227, 179, 79, 0.2), rgba(227, 179, 79, 0) 70%),
      #0f0c08;
  }
  .painel > :global(*) {
    grid-area: 1 / 1;
  }
  /* A rachadura: a divisa entre os dois deuses. Pulsa devagar e acende quando a ação está em foco. */
  .fenda {
    position: relative;
    z-index: 2;
    width: 2px;
    flex: none;
    background: linear-gradient(to bottom, #f6e3b0, #fff6df 45%, #cdb4ff);
    box-shadow:
      -10px 0 26px -4px rgba(227, 179, 79, 0.55),
      10px 0 26px -4px rgba(157, 114, 230, 0.6);
    animation: pulsa 5.5s ease-in-out infinite;
    transition:
      box-shadow 0.35s cubic-bezier(0.16, 1, 0.3, 1),
      transform 0.35s cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes pulsa {
    0%,
    100% {
      opacity: 0.72;
    }
    50% {
      opacity: 1;
    }
  }
  .cena:has(:global(.acao:is(:hover, :focus-visible))) .fenda,
  .cena.aberto .fenda {
    transform: scaleX(1.6);
    box-shadow:
      -18px 0 42px -2px rgba(227, 179, 79, 0.85),
      18px 0 42px -2px rgba(157, 114, 230, 0.9);
  }
</style>
