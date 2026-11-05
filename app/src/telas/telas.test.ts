import { render, screen, fireEvent } from "@testing-library/svelte";
import { it, expect, vi } from "vitest";
import Entrar from "./Entrar.svelte";
import Aguardando from "./Aguardando.svelte";
import Inicio from "./Inicio.svelte";
import Ajustes from "./Ajustes.svelte";
import Pasta from "./Pasta.svelte";
it("login troca o botão e permite cancelar", async () => {
  const cancelar = vi.fn();
  const { rerender } = render(Entrar, {
    ocupado: false,
    entrar: vi.fn(),
    cancelar,
  });
  expect(
    screen.getByRole("button", { name: "Entrar com Discord" }),
  ).toBeEnabled();
  await rerender({ ocupado: true, entrar: vi.fn(), cancelar });
  expect(screen.getByText("Aguardando o navegador…")).toBeVisible();
  await fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
  expect(cancelar).toHaveBeenCalled();
});
it("aguardando mostra ID copiável", () => {
  render(Aguardando, { id: "123456", copiar: vi.fn(), entrar: vi.fn() });
  expect(screen.getByText("123456")).toBeVisible();
  expect(screen.getByRole("button", { name: "Copiar" })).toBeEnabled();
});
it("instalação bloqueia jogar e mostra etapa", () => {
  render(Inicio, {
    ocupado: true,
    progresso: { etapa: "Baixando mods", feito: 14, total: 63 },
    status: null,
    atualizacao: true,
    jogar: vi.fn(),
  });
  expect(screen.getByRole("button", { name: "Jogar" })).toBeDisabled();
  expect(screen.getByText("Baixando mods 14/63")).toBeVisible();
  expect(screen.getByText("Atualização pronta")).toBeVisible();
});
it("mostra o status do servidor com jogadores", () => {
  render(Inicio, {
    ocupado: false,
    progresso: null,
    status: { online: true, jogadores: 3, max: 20, motd: "§6Relíquias §lSMP" },
    atualizacao: false,
    jogar: vi.fn(),
  });
  expect(screen.getByText("Online")).toBeVisible();
  expect(screen.getByText("3/20")).toBeVisible();
  expect(screen.getByRole("button", { name: "Jogar" })).toBeEnabled();
});
it("RAM restaura None para usar padrão do core", async () => {
  const salvar = vi.fn();
  render(Ajustes, {
    dados: {
      memoria_mb: 4096,
      memoria_efetiva: 4096,
      memoria_total: 7168,
      memoria_maxima: 5120,
      shader: false,
      visual: true,
      visual_bytes: 1.3 * 1024 ** 3,
      pasta: "C:\\Jogo",
    },
    salvar,
    mudar: vi.fn(),
    verificar: vi.fn(),
    abrir: vi.fn(),
    ocupado: false,
  });
  expect(screen.getByText("Menos de 8 GB de RAM")).toBeVisible();
  expect(screen.getByRole("slider")).toHaveAttribute("max", "5120");
  await fireEvent.click(
    screen.getByRole("button", { name: "Restaurar padrão" }),
  );
  expect(salvar).toHaveBeenCalledWith({ memoria_mb: null, shader: false, visual: true });
});
it.each(["NaoVazia", "Sincronizada"])(
  "aviso %s não bloqueia instalar",
  (aviso) => {
    render(Pasta, {
      dados: {
        caminho: "C:\\Documents\\Reliquias",
        livre: 100 * 1024 ** 3,
        estimado: 8 * 1024 ** 3,
        aviso,
        erro: null,
        visual: true,
        visual_bytes: 1.3 * 1024 ** 3,
      },
      mudando: false,
      ocupado: false,
      escolher: vi.fn(),
      confirmar: vi.fn(),
      abrir: vi.fn(),
    });
    expect(screen.getByRole("button", { name: "Instalar" })).toBeEnabled();
    expect(screen.getByText("C:\\Documents\\Reliquias")).toBeVisible();
    expect(screen.getByText(/100 GB livres/)).toBeVisible();
  },
);
it("erro de pasta bloqueia instalar", () => {
  render(Pasta, {
    dados: {
      caminho: "C:\\Jogo",
      livre: 0,
      estimado: 8 * 1024 ** 3,
      aviso: null,
      erro: "SemEspaco",
      visual: true,
      visual_bytes: 1.3 * 1024 ** 3,
    },
    mudando: false,
    ocupado: false,
    escolher: vi.fn(),
    confirmar: vi.fn(),
    abrir: vi.fn(),
  });
  expect(screen.getByRole("button", { name: "Instalar" })).toBeDisabled();
  expect(screen.getByText("Espaço insuficiente")).toBeVisible();
});
it("mudança oferece mover ou reinstalar", () => {
  render(Pasta, {
    dados: {
      caminho: "C:\\Jogo",
      livre: 100 * 1024 ** 3,
      estimado: 8 * 1024 ** 3,
      aviso: null,
      erro: null,
      visual: true,
      visual_bytes: 1.3 * 1024 ** 3,
    },
    mudando: true,
    ocupado: false,
    escolher: vi.fn(),
    confirmar: vi.fn(),
    abrir: vi.fn(),
  });
  expect(
    screen.getByRole("button", { name: "Mover os arquivos" }),
  ).toBeVisible();
  expect(
    screen.getByRole("button", { name: "Instalar de novo" }),
  ).toBeVisible();
});
it("pacote visual: marcado por padrão, desmarcar vale na instalação", async () => {
  const confirmar = vi.fn();
  render(Pasta, {
    dados: {
      caminho: "C:\Jogo",
      livre: 100 * 1024 ** 3,
      estimado: 8 * 1024 ** 3,
      aviso: null,
      erro: null,
      visual: true,
      visual_bytes: 1.3 * 1024 ** 3,
    },
    mudando: false,
    ocupado: false,
    escolher: vi.fn(),
    confirmar,
    abrir: vi.fn(),
  });
  const marcador = screen.getByRole("checkbox", { name: /Pacote visual/ });
  expect(marcador).toBeChecked();
  expect(screen.getByText(/1,3 GB/)).toBeVisible();
  await fireEvent.click(marcador);
  await fireEvent.click(screen.getByRole("button", { name: "Instalar" }));
  expect(confirmar).toHaveBeenCalledWith(false, false);
});
it("ajustes: shader independe do pacote visual", async () => {
  const salvar = vi.fn();
  render(Ajustes, {
    dados: {
      memoria_mb: null,
      memoria_efetiva: 6144,
      memoria_total: 16384,
      memoria_maxima: 14336,
      shader: true,
      visual: false,
      visual_bytes: 1.3 * 1024 ** 3,
      pasta: "C:\Jogo",
    },
    salvar,
    mudar: vi.fn(),
    verificar: vi.fn(),
    abrir: vi.fn(),
    ocupado: false,
  });
  const shader = screen.getByRole("checkbox", { name: /Shader/ });
  expect(shader).toBeEnabled();
  expect(shader).toBeChecked();
  await fireEvent.click(shader);
  expect(salvar).toHaveBeenCalledWith({ memoria_mb: null, shader: false, visual: false });
  await fireEvent.click(screen.getByRole("checkbox", { name: /Pacote visual/ }));
  expect(salvar).toHaveBeenCalledWith({ memoria_mb: null, shader: true, visual: true });
});
