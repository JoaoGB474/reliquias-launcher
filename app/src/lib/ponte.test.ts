import { describe, it, expect, vi } from "vitest";
import { criarPonte, mensagemErro, mensagemSaida } from "./ponte";
describe("ponte", () => {
  it("serializa apenas ajustes editáveis", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    await criarPonte(invoke).salvar_ajustes({ memoria_mb: null, shader: true, visual: true });
    expect(invoke).toHaveBeenCalledWith("salvar_ajustes", {
      ajustes: { memoria_mb: null, shader: true, visual: true },
    });
  });
  it("traduz erros sem expor dados internos", () => {
    expect(mensagemErro({ codigo: "Rede" })).toBe("Sem conexão com o servidor");
    expect(mensagemErro({ codigo: "SemEspaco" })).toBe("Espaço insuficiente");
    expect(mensagemErro("token secreto")).toBe("Não foi possível concluir");
  });
  it("distingue a saída do jogo de instalação", () => {
    expect(mensagemSaida(7)).toBe("O jogo fechou com erro (código 7)");
    expect(mensagemSaida(0)).toBe("");
  });
});
