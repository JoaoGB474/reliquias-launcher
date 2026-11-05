import type {
  Ponte,
  Novidades,
  Estado,
  Progresso,
  Status,
  Ajustes,
  PastaInfo,
} from "./tipos";
import { mensagemErro, mensagemSaida } from "./ponte";
export function criarEstado(ponte: Ponte) {
  let estado = $state<Estado>({
    tela: "entrar",
    discord_id: "",
    ocupado: false,
  });
  let progresso = $state<Progresso | null>(null),
    status = $state<Status | null>(null),
    ajustes = $state<Ajustes | null>(null),
    pasta = $state<PastaInfo | null>(null);
  let erro = $state(""),
    atualizacao = $state(false),
    pagina = $state<"principal" | "ajustes" | "pasta">("principal"),
    carregando = $state(true);
  let repetir: () => Promise<unknown> = async () => {};
  let fechado = false;
  let novidades = $state<Novidades>({ servidor: [], modpack: [] });
  async function atualizarNovidades() {
    try {
      const resposta = await ponte.novidades();
      if (!fechado) novidades = resposta;
    } catch {
      if (!fechado) novidades = { servidor: [], modpack: [] };
    }
  }
  const disposers: (() => void)[] = [];
  async function executar(fn: () => Promise<unknown>) {
    erro = "";
    repetir = fn;
    try {
      await fn();
    } catch (e) {
      erro = mensagemErro(e);
    }
  }
  async function atualizarEstado(e: Estado) {
    estado = e;
    if (!e.ocupado) progresso = null;
    if (e.tela === "pasta" && !pasta) pasta = await ponte.sugerir_pasta();
  }
  async function atualizarStatus() {
    try {
      status = await ponte.status();
    } catch (e) {
      status = null;
      if (!erro && !estado.ocupado) {
        erro = mensagemErro(e);
        repetir = atualizarStatus;
      }
    }
  }
  async function iniciar() {
    const buscaNovidades = atualizarNovidades();
    if (!fechado) {
      const id = setInterval(() => void atualizarNovidades(), 600000);
      disposers.push(() => clearInterval(id));
    }
    await executar(async () => {
      const registrar = async <K extends Parameters<Ponte["ouvir"]>[0]>(
        nome: K,
        fn: Parameters<Ponte["ouvir"]>[1],
      ) => {
        const off = await ponte.ouvir(nome, fn);
        if (fechado) off();
        else disposers.push(off);
      };
      await registrar("estado", (e) => {
        void atualizarEstado(e as Estado).catch((e) => {
          erro = mensagemErro(e);
        });
      });
      await registrar("progresso", (e) => {
        progresso = e as Progresso;
      });
      await registrar("jogo_saiu", (e) => {
        erro = mensagemSaida((e as { codigo: number | null }).codigo);
        progresso = null;
        repetir = () => ponte.jogar();
      });
      await registrar("erro", (e) => {
        erro = mensagemErro(e);
      });
      await registrar("atualizacao_pronta", () => {
        atualizacao = true;
      });
      await atualizarEstado(await ponte.estado());
      carregando = false;
      await atualizarStatus();
    });
    carregando = false;
    if (!fechado) {
      const id = setInterval(() => {
        void atualizarStatus();
      }, 30000);
      disposers.push(() => clearInterval(id));
    }
    await buscaNovidades;
  }
  return {
    get novidades() {
      return novidades;
    },
    get estado() {
      return estado;
    },
    get progresso() {
      return progresso;
    },
    get status() {
      return status;
    },
    get ajustes() {
      return ajustes;
    },
    get pasta() {
      return pasta;
    },
    get erro() {
      return erro;
    },
    get atualizacao() {
      return atualizacao;
    },
    get pagina() {
      return pagina;
    },
    get carregando() {
      return carregando;
    },
    iniciar,
    destruir: () => {
      fechado = true;
      disposers.splice(0).forEach((f) => f());
    },
    executar,
    tentar: () => executar(repetir),
    entrar: () => executar(() => ponte.entrar()),
    cancelar: () => executar(() => ponte.cancelar()),
    sair: () =>
      executar(async () => {
        await ponte.sair();
        pagina = "principal";
      }),
    jogar: () => executar(() => ponte.jogar()),
    abrirAjustes: () =>
      executar(async () => {
        ajustes = await ponte.ajustes();
        pagina = "ajustes";
      }),
    voltar: () => {
      pagina = "principal";
    },
    salvar: (a: Parameters<Ponte["salvar_ajustes"]>[0]) =>
      executar(async () => {
        const mudouVisual = (ajustes ?? await ponte.ajustes()).visual !== a.visual;
        await ponte.salvar_ajustes(a);
        if (mudouVisual) {
          repetir = () => ponte.verificar_arquivos();
          await ponte.verificar_arquivos();
        }
        ajustes = await ponte.ajustes();
      }),
    mudar: () =>
      executar(async () => {
        pasta = await ponte.sugerir_pasta();
        pagina = "pasta";
      }),
    escolher: () =>
      executar(async () => {
        const p = await ponte.escolher_pasta();
        if (p) pasta = p;
      }),
    confirmar: (mover: boolean, visual: boolean) =>
      executar(async () => {
        if (!pasta) return;
        await ponte.definir_pasta(pasta.caminho, mover, visual);
        await atualizarEstado(await ponte.estado());
        pagina = "principal";
      }),
    abrir: (caminho?: string) => executar(() => ponte.abrir_pasta(caminho)),
    verificar: () => executar(() => ponte.verificar_arquivos()),
    copiar: () =>
      executar(() => navigator.clipboard.writeText(estado.discord_id)),
  };
}

if (import.meta.vitest) {
  const { it, expect, vi } = import.meta.vitest;
  it("busca novidades ao iniciar, a cada dez minutos e para ao destruir", async () => {
    vi.useFakeTimers();
    const { criarMock } = await import("./ponte.mock");
    const { novidadesMock } = await import("./novidades");
    const ponte = criarMock("entrar");
    ponte.novidades = vi.fn().mockResolvedValue(novidadesMock);
    const app = criarEstado(ponte);
    try {
      expect(app.novidades).toEqual({ servidor: [], modpack: [] });
      await app.iniciar();
      expect(app.novidades).toEqual(novidadesMock);
      expect(ponte.novidades).toHaveBeenCalledTimes(1);
      await vi.advanceTimersByTimeAsync(599999);
      expect(ponte.novidades).toHaveBeenCalledTimes(1);
      await vi.advanceTimersByTimeAsync(1);
      expect(ponte.novidades).toHaveBeenCalledTimes(2);
      app.destruir();
      await vi.advanceTimersByTimeAsync(600000);
      expect(ponte.novidades).toHaveBeenCalledTimes(2);
    } finally {
      app.destruir();
      vi.useRealTimers();
    }
  });
  it("falha de novidades mantém listas vazias sem falhar a tela e permite recuperação", async () => {
    vi.useFakeTimers();
    const { criarMock } = await import("./ponte.mock");
    const { novidadesMock } = await import("./novidades");
    const ponte = criarMock("entrar");
    ponte.novidades = vi
      .fn()
      .mockRejectedValueOnce({ codigo: "Rede" })
      .mockResolvedValue(novidadesMock);
    const app = criarEstado(ponte);
    try {
      await app.iniciar();
      expect(app.novidades).toEqual({ servidor: [], modpack: [] });
      expect(app.erro).toBe("");
      expect(app.carregando).toBe(false);
      await vi.advanceTimersByTimeAsync(600000);
      expect(app.novidades).toEqual(novidadesMock);
    } finally {
      app.destruir();
      vi.useRealTimers();
    }
  });
}
