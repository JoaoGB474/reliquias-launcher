import type { Ponte, Estado, Eventos, Ajustes, PastaInfo } from "./tipos";
import { novidadesMock } from "./novidades";
export function criarMock(cenario: string): Ponte {
  let e: Estado = {
    tela:
      cenario === "entrar"
        ? "entrar"
        : cenario === "aguardando"
          ? "aguardando"
          : cenario === "pasta"
            ? "pasta"
            : "inicio",
    discord_id: "123456789012345678",
    ocupado: cenario === "instalando",
  };
  let a: Ajustes = {
    memoria_mb: null,
    memoria_efetiva: 6144,
    memoria_total: 16384,
    memoria_maxima: 14336,
    shader: true,
    visual: true,
    visual_bytes: 1_300_000_000,
    pasta: "C:\\Users\\Jogador\\Documents\\Reliquias",
  };
  let pasta: PastaInfo = {
    caminho: a.pasta,
    visual: a.visual,
    visual_bytes: a.visual_bytes,
    livre: 128 * 1024 ** 3,
    estimado: 8 * 1024 ** 3,
    aviso: null,
    erro: null,
  };
  const listeners = new Map<string, Set<(e: never) => void>>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  function emitir<K extends keyof Eventos>(nome: K, p: Eventos[K]) {
    listeners.get(nome)?.forEach((fn) => fn(p as never));
  }
  function estado() {
    emitir("estado", { ...e });
  }
  function progresso() {
    let feito = 14;
    e.ocupado = true;
    estado();
    const passo = () => {
      emitir("progresso", { etapa: "Baixando mods", feito, total: 63 });
      if (feito++ < 63) timer = setTimeout(passo, 1000);
      else {
        e.ocupado = false;
        estado();
        emitir("jogo_saiu", { codigo: 0 });
      }
    };
    passo();
  }
  return {
    estado: async () => ({ ...e }),
    novidades: async () => novidadesMock,
    ouvir: async (nome, fn) => {
      if (!listeners.has(nome)) listeners.set(nome, new Set());
      listeners.get(nome)!.add(fn as (e: never) => void);
      if (nome === "progresso" && cenario === "instalando")
        queueMicrotask(progresso);
      if (nome === "erro" && cenario === "erro")
        queueMicrotask(() => emitir("erro", { codigo: "Rede" }));
      if (nome === "atualizacao_pronta" && cenario === "atualizacao")
        queueMicrotask(() => emitir("atualizacao_pronta", null));
      return () => {
        listeners.get(nome)?.delete(fn as (e: never) => void);
        if (![...listeners.values()].some((s) => s.size)) clearTimeout(timer);
      };
    },
    entrar: async () => {
      e.ocupado = true;
      estado();
      timer = setTimeout(() => {
        e = { ...e, tela: "pasta", ocupado: false };
        estado();
      }, 2500);
    },
    cancelar: async () => {
      clearTimeout(timer);
      e.ocupado = false;
      estado();
    },
    sair: async () => {
      clearTimeout(timer);
      e = { ...e, tela: "entrar", ocupado: false };
      estado();
    },
    jogar: async () => progresso(),
    verificar_arquivos: async () => progresso(),
    ajustes: async () => ({ ...a }),
    salvar_ajustes: async (v) => {
      a = { ...a, ...v, memoria_efetiva: v.memoria_mb ?? 6144 };
      pasta.visual = a.visual;
    },
    abrir_pasta: async () => {},
    gerar_relatorio: async () => {
      throw { codigo: "RelatorioIndisponivel" };
    },
    status: async () => {
      if (cenario === "erro") throw { codigo: "Rede" };
      return { online: true, jogadores: 12, max: 40, motd: "§6Relíquias SMP" };
    },
    sugerir_pasta: async () => ({ ...pasta }),
    escolher_pasta: async () => {
      pasta = {
        ...pasta,
        caminho: "D:\\Jogos\\Reliquias",
        aviso: "Sincronizada",
      };
      return { ...pasta };
    },
    validar_pasta: async (caminho) => ({ ...pasta, caminho }),
    definir_pasta: async (caminho, _mover, visual) => {
      a.visual = visual;
      pasta.visual = visual;
      a.pasta = caminho;
      e.tela = "inicio";
      estado();
    },
  };
}

if (import.meta.vitest) {
  const { it, expect } = import.meta.vitest;
  it("oferece novidades de demonstração sem login", async () => {
    const { novidadesMock } = await import("./novidades");
    expect(await criarMock("entrar").novidades()).toEqual(novidadesMock);
  });
}
