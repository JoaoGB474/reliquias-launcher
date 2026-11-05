declare global {
  interface ImportMeta {
    readonly vitest?: typeof import("vitest");
  }
}
export interface AjustesEditaveis {
  memoria_mb: number | null;
  shader: boolean;
  visual: boolean;
}
export interface Ajustes extends AjustesEditaveis {
  visual_bytes: number;
  memoria_efetiva: number;
  memoria_total: number;
  memoria_maxima: number;
  pasta: string;
}
export interface Estado {
  tela: "entrar" | "aguardando" | "inicio" | "pasta";
  discord_id: string;
  ocupado: boolean;
}
export interface Progresso {
  etapa: string;
  feito: number;
  total: number;
}
export interface Status {
  online: boolean;
  jogadores: number;
  max: number;
  motd: string;
}
export interface PastaInfo {
  visual: boolean;
  visual_bytes: number;
  caminho: string;
  livre: number;
  estimado: number;
  aviso: string | null;
  erro: string | null;
}
export interface Eventos {
  progresso: Progresso;
  jogo_saiu: { codigo: number | null };
  atualizacao_pronta: null;
  erro: { codigo: string; mensagem?: string };
  estado: Estado;
}
export interface Ponte {
  novidades(): Promise<Novidades>;
  estado(): Promise<Estado>;
  entrar(): Promise<void>;
  sair(): Promise<void>;
  jogar(): Promise<void>;
  cancelar(): Promise<void>;
  verificar_arquivos(): Promise<void>;
  ajustes(): Promise<Ajustes>;
  salvar_ajustes(a: AjustesEditaveis): Promise<void>;
  abrir_pasta(caminho?: string): Promise<void>;
  gerar_relatorio(): Promise<string>;
  status(): Promise<Status>;
  sugerir_pasta(): Promise<PastaInfo>;
  escolher_pasta(): Promise<PastaInfo | null>;
  validar_pasta(caminho: string): Promise<PastaInfo>;
  definir_pasta(caminho: string, mover: boolean, visual: boolean): Promise<void>;
  ouvir<K extends keyof Eventos>(
    nome: K,
    fn: (e: Eventos[K]) => void,
  ): Promise<() => void>;
}

export interface Novidade {
  versao: number;
  data: string;
  titulo: string;
  itens: string[];
}
export interface Novidades {
  servidor: Novidade[];
  modpack: Novidade[];
}
