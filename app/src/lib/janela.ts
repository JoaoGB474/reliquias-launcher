// Controles da janela sem moldura. No navegador (modo dev) não fazem nada.
const noTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function janela() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

export async function minimizar(): Promise<void> {
  if (noTauri()) await (await janela()).minimize();
}

export async function fechar(): Promise<void> {
  if (noTauri()) await (await janela()).close();
}
