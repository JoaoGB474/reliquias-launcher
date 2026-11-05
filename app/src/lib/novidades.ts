// Novidades do servidor e do modpack. Vêm do launcher (fatos do manifesto + texto opcional da staff).
export interface Novidade {
  versao: number;
  data: string; // AAAA-MM-DD
  titulo: string;
  itens: string[];
}
export interface Novidades {
  servidor: Novidade[];
  modpack: Novidade[];
}

// Só para o modo dev no navegador (ponte de mentira).
export const novidadesMock: Novidades = {
  servidor: [
    {
      versao: 5,
      data: "2026-11-05",
      titulo: "Lobby novo e /home liberado",
      itens: [
        "Ilha do lobby refeita, com spawn fixo",
        "/home, /tpa e /warp ativados, até 3 homes",
        "Chat de voz por proximidade na porta 24304",
      ],
    },
    {
      versao: 4,
      data: "2026-11-02",
      titulo: "Loot das relíquias mais difícil",
      itens: ["Relíquias só em baús de estruturas raras", "Lootr nos baús: cada jogador tem o seu"],
    },
    {
      versao: 3,
      data: "2026-10-30",
      titulo: "Capítulo I no livro de quests",
      itens: ["29 capítulos e 593 quests", "Raças e classes da lore"],
    },
  ],
  modpack: [
    {
      versao: 5,
      data: "2026-11-05",
      titulo: "EMI no lugar do JEI",
      itens: [
        "Adicionados: EMI, EMI Loot, EMI Ores, TooManyRecipeViewers",
        "Removido: JEI",
        "Sodium: opções de clima corrigidas",
      ],
    },
    {
      versao: 4,
      data: "2026-11-03",
      titulo: "Occultism e Mystical Agriculture",
      itens: ["Adicionados: Occultism, Mystical Agriculture e addons", "Atualizado: Create 6.0"],
    },
  ],
};

export function dataCurta(iso: string): string {
  const [, m, d] = iso.split("-");
  return `${d}/${m}`;
}
