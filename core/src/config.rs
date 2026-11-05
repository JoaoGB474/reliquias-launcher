use crate::erro::{Erro, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub pasta: Option<PathBuf>,
    pub memoria_mb: Option<u32>,
    pub shader: bool,
    pub visual: bool,
}
pub const MEMORIA_PADRAO_MB: u32 = 6144;
pub const MEMORIA_MIN_MB: u32 = 2048;
pub const MEMORIA_MAX_MB: u32 = 65536;

impl Default for Config {
    fn default() -> Self {
        Self {
            pasta: None,
            memoria_mb: None,
            shader: false,
            visual: true,
        }
    }
}

impl Config {
    /// RAM do jogo: o que o jogador escolheu ou 6 GB. Nunca abaixo de 2 GB (o pack não carrega).
    pub fn memoria_efetiva(&self) -> u32 {
        self.memoria_mb
            .unwrap_or(MEMORIA_PADRAO_MB)
            .clamp(MEMORIA_MIN_MB, MEMORIA_MAX_MB)
    }
    /// RAM do jogo neste PC: a escolha do jogador (ou 6 GB), sempre deixando 2 GB livres pro Windows.
    /// `total_mb` = RAM total do PC (0 se não deu pra descobrir: aí vale o padrão sem teto).
    pub fn memoria_para(&self, total_mb: u32) -> u32 {
        let base = self.memoria_efetiva();
        if total_mb == 0 {
            return base;
        }
        let teto = total_mb.saturating_sub(2048).max(MEMORIA_MIN_MB);
        base.min(teto)
    }
    /// Valida e grava a escolha do jogador (2 GB a 64 GB). `None` volta ao padrão de 6 GB.
    pub fn definir_memoria(&mut self, mb: Option<u32>) -> Result<()> {
        if mb.is_some_and(|v| !(MEMORIA_MIN_MB..=MEMORIA_MAX_MB).contains(&v)) {
            return Err(Erro::ConfiguracaoInvalida);
        }
        self.memoria_mb = mb;
        Ok(())
    }
    pub fn caminho() -> Result<PathBuf> {
        Ok(
            PathBuf::from(std::env::var_os("APPDATA").ok_or(Erro::ConfiguracaoInvalida)?)
                .join("Reliquias-Launcher/config.json"),
        )
    }
    pub fn carregar() -> Result<Self> {
        Self::carregar_de(&Self::caminho()?)
    }
    pub fn carregar_de(p: &Path) -> Result<Self> {
        match std::fs::read(p) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn salvar(&self) -> Result<()> {
        self.salvar_em(&Self::caminho()?)
    }
    pub fn salvar_em(&self, p: &Path) -> Result<()> {
        let pai = p.parent().ok_or(Erro::CaminhoInvalido)?;
        std::fs::create_dir_all(pai)?;
        let mut tmp = tempfile::NamedTempFile::new_in(pai)?;
        tmp.write_all(&serde_json::to_vec_pretty(self)?)?;
        tmp.as_file().sync_all()?;
        tmp.persist(p).map_err(|e| Erro::Io(e.error))?;
        Ok(())
    }
}
