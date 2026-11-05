<script lang="ts">
  import { onMount } from "svelte";
  import { selecionarPonte } from "./lib/ponte";
  import { criarEstado } from "./lib/estado.svelte";
    import { VERSAO } from "./lib/versao";
  import Barra from "./lib/Barra.svelte";
  import Cena from "./lib/Cena.svelte";
  import Entrar from "./telas/Entrar.svelte";
  import Aguardando from "./telas/Aguardando.svelte";
  import Inicio from "./telas/Inicio.svelte";
  import Ajustes from "./telas/Ajustes.svelte";
  import Pasta from "./telas/Pasta.svelte";
  let app = $state<ReturnType<typeof criarEstado> | null>(null);
  // O mundo "se abre" quando o jogador passa do login.
  let aberto = $derived(
    !!app && !app.carregando && app.estado.tela !== "entrar" && app.estado.tela !== "pasta",
  );
  onMount(() => {
    let fechado = false;
    void selecionarPonte().then((p) => {
      if (fechado) return;
      app = criarEstado(p);
      void app.iniciar();
    });
    const oculto = () => {
      if (document.hidden) document.documentElement.dataset.oculto = "";
      else delete document.documentElement.dataset.oculto;
    };
    document.addEventListener("visibilitychange", oculto);
    return () => {
      fechado = true;
      app?.destruir();
      document.removeEventListener("visibilitychange", oculto);
    };
  });
</script>

<div class="janela">
  <Barra versao={VERSAO} />
  <Cena
    novidades={app?.novidades ?? { servidor: [], modpack: [] }}
    {aberto}
    offline={!!app && !app.carregando && !app.status?.online}
  >
    {#if !app || app.carregando}
      <p class="carregando" role="status">Carregando…</p>
    {:else if app.pagina === "ajustes" && app.ajustes}
      <Ajustes
        dados={app.ajustes}
        ocupado={app.estado.ocupado}
        salvar={app.salvar}
        mudar={app.mudar}
        verificar={app.verificar}
        abrir={() => app?.abrir()}
        voltar={app.voltar}
        progresso={app.progresso}
        erro={app.erro}
        tentar={app.tentar}
      />
    {:else if (app.pagina === "pasta" || app.estado.tela === "pasta") && app.pasta}
      <Pasta
        dados={app.pasta}
        ocupado={app.estado.ocupado}
        mudando={app.pagina === "pasta"}
        escolher={app.escolher}
        confirmar={app.confirmar}
        abrir={() => app?.abrir(app.pasta?.caminho)}
        voltar={app.voltar}
      />
    {:else if app.estado.tela === "entrar"}
      <Entrar
        ocupado={app.estado.ocupado}
        entrar={app.entrar}
        cancelar={app.cancelar}
        status={app.status}
        erro={app.erro}
        tentar={app.tentar}
      />
    {:else if app.estado.tela === "aguardando"}
      <Aguardando id={app.estado.discord_id} copiar={app.copiar} entrar={app.entrar} />
    {:else}
      <Inicio
        ocupado={app.estado.ocupado}
        progresso={app.progresso}
        status={app.status}
        atualizacao={app.atualizacao}
        jogar={app.jogar}
        ajustes={app.abrirAjustes}
        sair={app.sair}
        erro={app.erro}
        tentar={app.tentar}
      />
    {/if}
  </Cena>
</div>

<style>
  .carregando {
    align-self: end;
    margin: 0;
    font-family: var(--pixel);
    font-size: 17px;
    color: var(--mudo);
  }
</style>
