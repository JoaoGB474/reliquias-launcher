import type { Ponte, AjustesEditaveis, Eventos } from "./tipos";
export const textos: Record<string, string> = {
  Rede: "Sem conexão com o servidor",
  Http: "Servidor indisponível",
  SemEspaco: "Espaço insuficiente",
  SemPermissao: "Sem permissão para esta pasta",
  CaminhoInvalido: "Escolha outra pasta",
  ConfiguracaoInvalida: "Configuração inválida",
  Instalacao: "Falha na instalação do jogo",
  ContaRevogada: "Entre novamente",
  SemAprovacao: "Aguardando aprovação",
  PortasOcupadas: "Portas de login ocupadas",
  CallbackInvalido: "Retorno do Discord inválido",
  LoginExpirado: "Login expirado",
  DiscordNaoConfigurado: "Discord não configurado",
  Cofre: "Cofre do sistema indisponível",
  JaAberto: "Uma operação já está em andamento",
  HashInvalido: "Arquivo corrompido",
  ServidorDesconhecido: "Servidor não reconhecido",
  EnderecoInvalido: "Endereço inválido",
  Io: "Não foi possível acessar os arquivos",
  Json: "Resposta inválida do servidor",
  AssinaturaInvalida: "Assinatura inválida",
  AtualizacaoNaoConfigurada: "Atualização não configurada",
  NaoVazia: "A pasta contém arquivos",
  Sincronizada: "Pasta sincronizada na nuvem",
  RelatorioIndisponivel: "Relatório ainda indisponível",
};
export function mensagemErro(e: unknown): string {
  const codigo =
    typeof e === "object" && e !== null && "codigo" in e
      ? String(e.codigo)
      : "";
  return textos[codigo] ?? "Não foi possível concluir";
}
export function mensagemSaida(codigo: number | null): string {
  return codigo === 0
    ? ""
    : codigo === null
      ? "O jogo fechou com erro"
      : "O jogo fechou com erro (código " + codigo + ")";
}
type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
export function criarPonte(
  invoke: Invoke,
  listen?: <K extends keyof Eventos>(
    nome: K,
    fn: (e: Eventos[K]) => void,
  ) => Promise<() => void>,
): Ponte {
  return {
    estado: () => invoke("estado"),
    entrar: () => invoke("entrar"),
    sair: () => invoke("sair"),
    jogar: () => invoke("jogar"),
    cancelar: () => invoke("cancelar"),
    verificar_arquivos: () => invoke("verificar_arquivos"),
    ajustes: () => invoke("ajustes"),
    salvar_ajustes: ({ memoria_mb, shader, visual }: AjustesEditaveis) =>
      invoke("salvar_ajustes", { ajustes: { memoria_mb, shader, visual } }),
    abrir_pasta: (caminho) =>
      invoke("abrir_pasta", { caminho: caminho ?? null }),
    gerar_relatorio: () => invoke("gerar_relatorio"),
    status: () => invoke("status"),
    novidades: () => invoke("novidades"),
    sugerir_pasta: () => invoke("sugerir_pasta"),
    escolher_pasta: () => invoke("escolher_pasta"),
    validar_pasta: (caminho) => invoke("validar_pasta", { caminho }),
    definir_pasta: (caminho, mover, visual) =>
      invoke("definir_pasta", { caminho, mover, visual }),
    ouvir: listen ?? (async () => () => {}),
  };
}
export async function selecionarPonte(): Promise<Ponte> {
  if (!("__TAURI_INTERNALS__" in window))
    return (await import("./ponte.mock")).criarMock(
      new URLSearchParams(location.search).get("cenario") ?? "entrar",
    );
  const [{ invoke }, { listen }] = await Promise.all([
    import("@tauri-apps/api/core"),
    import("@tauri-apps/api/event"),
  ]);
  return criarPonte(invoke, (nome, fn) =>
    listen(nome, (e) => fn(e.payload as never)),
  );
}

if (import.meta.vitest) {
  const { it, expect, vi } = import.meta.vitest;
  it("invoca novidades sem argumentos de autenticação", async () => {
    const dados = { servidor: [], modpack: [] };
    const invoke = vi.fn().mockResolvedValue(dados);
    expect(await criarPonte(invoke).novidades()).toEqual(dados);
    expect(invoke).toHaveBeenCalledExactlyOnceWith("novidades");
  });
}
