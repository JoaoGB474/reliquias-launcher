import { it, expect, vi } from "vitest";
import { criarEstado } from "./estado.svelte";
import { criarMock } from "./ponte.mock";
it("conserva erro de saída após estado desocupado e permite repetir jogar", async () => {
  const p = criarMock("atualizacao");
  const handlers: Record<string, (e: any) => void> = {};
  p.ouvir = vi.fn(async (n, f) => {
    handlers[n] = f;
    return () => {};
  });
  p.jogar = vi.fn(async () => {});
  const app = criarEstado(p);
  await app.iniciar();
  handlers.jogo_saiu({ codigo: 7 });
  handlers.estado({ tela: "inicio", discord_id: "1", ocupado: false });
  await Promise.resolve();
  expect(app.erro).toBe("O jogo fechou com erro (código 7)");
  await app.tentar();
  expect(p.jogar).toHaveBeenCalledOnce();
  app.destruir();
});
it("status lento não prende a abertura em carregando", async () => {
  const p = criarMock("pasta");
  let resolver: () => void = () => {};
  p.status = () =>
    new Promise((r) => {
      resolver = () => r({ online: true, jogadores: 0, max: 20, motd: "" });
    });
  const app = criarEstado(p);
  const inicio = app.iniciar();
  await vi.waitFor(() => expect(app.carregando).toBe(false));
  expect(app.carregando).toBe(false);
  expect(app.pasta?.caminho).toContain("Reliquias");
  resolver();
  await inicio;
  app.destruir();
});
it("cancela login simulado sem transição tardia", async () => {
  vi.useFakeTimers();
  const p = criarMock("entrar");
  await p.entrar();
  await p.cancelar();
  await vi.advanceTimersByTimeAsync(4000);
  expect((await p.estado()).tela).toBe("entrar");
  expect((await p.estado()).ocupado).toBe(false);
  vi.useRealTimers();
});

it("falha de status não substitui a tentativa de instalação", async () => {
 vi.useFakeTimers();
 const p=criarMock("atualizacao");p.jogar=vi.fn().mockRejectedValue({codigo:"HashInvalido"});
 const app=criarEstado(p);
 try {
  await app.iniciar();await app.jogar();
  p.status=vi.fn().mockRejectedValue({codigo:"Rede"});await vi.advanceTimersByTimeAsync(30000);
  expect(app.erro).toBe("Arquivo corrompido");await app.tentar();expect(p.jogar).toHaveBeenCalledTimes(2);
 } finally {app.destruir();vi.useRealTimers();}
});

it("verifica arquivos somente quando muda o pacote visual", async () => {
 const p=criarMock("inicio"); p.verificar_arquivos=vi.fn().mockResolvedValue(undefined);
 const app=criarEstado(p); await app.abrirAjustes();
 await app.salvar({memoria_mb:null,shader:true,visual:true}); expect(p.verificar_arquivos).not.toHaveBeenCalled();
 await app.salvar({memoria_mb:null,shader:true,visual:false}); expect(p.verificar_arquivos).toHaveBeenCalledOnce();
});
it("confirma a pasta com a escolha visual", async () => {
 const p=criarMock("pasta"); p.definir_pasta=vi.fn().mockResolvedValue(undefined);
 const app=criarEstado(p); await app.mudar(); await app.confirmar(false,false);
 expect(p.definir_pasta).toHaveBeenCalledWith(app.pasta?.caminho,false,false);
});
