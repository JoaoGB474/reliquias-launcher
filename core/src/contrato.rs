use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Novidades {
    pub servidor: Vec<Novidade>,
    pub modpack: Vec<Novidade>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Novidade {
    pub versao: u64,
    pub data: String,
    pub titulo: String,
    pub itens: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Perfil {
    pub id: String,
    pub nick: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Conta {
    pub discord_id: String,
    pub nome_discord: String,
    pub perfil: Option<Perfil>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct LoginResp {
    pub refresh: String,
    pub access: String,
    pub expira_em: u64,
    pub conta: Conta,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    pub nome: String,
    pub sha256: String,
    pub url: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Origem {
    Cdn,
    Servidor,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Arquivo {
    pub caminho: String,
    pub sha256: String,
    pub tamanho: u64,
    pub origem: Origem,
    pub url: String,
    pub uma_vez: bool,
    #[serde(default = "grupo_base")]
    pub grupo: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServidorJogo {
    pub nome: String,
    pub endereco: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifesto {
    #[serde(default)]
    pub servidor: Option<ServidorJogo>,
    pub versao: u64,
    pub minecraft: String,
    pub neoforge: String,
    pub java: u8,
    pub agent: Agent,
    pub arquivos: Vec<Arquivo>,
    pub pastas_gerenciadas: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Status {
    pub online: bool,
    pub jogadores: u32,
    pub max: u32,
    pub motd: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErroApi {
    pub erro: String,
}

fn grupo_base() -> String {
    "base".into()
}
impl Manifesto {
    pub fn filtrado(&self, visual: bool) -> Self {
        let mut m = self.clone();
        if !visual {
            m.arquivos.retain(|a| a.grupo != "visual");
        }
        m
    }
}
