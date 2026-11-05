use crate::{
    baixar::Hash,
    cliente::Cliente,
    contrato::Manifesto,
    erro::{Erro, Result},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
pub type Versao = semver::Version;
#[derive(Clone, Deserialize, Serialize)]
pub struct Ultimo {
    pub versao: String,
    pub sha256: String,
    pub tamanho: u64,
    pub assinatura: String,
}
pub struct Pronto {
    bytes: Vec<u8>,
    meta: Ultimo,
    chave: [u8; 32],
}
impl Pronto {
    pub fn aplicar(self) -> Result<()> {
        verificar(&self.meta, &self.bytes, &self.chave)?;
        let mut file = tempfile::NamedTempFile::new()?;
        use std::io::Write;
        file.write_all(&self.bytes)?;
        file.as_file().sync_all()?;
        self_replace::self_replace(file.path())?;
        std::process::Command::new(std::env::current_exe()?).spawn()?;
        Ok(())
    }
}
pub fn verificar(meta: &Ultimo, bytes: &[u8], chave: &[u8; 32]) -> Result<()> {
    if meta.tamanho != bytes.len() as u64
        || !Hash::sha256(&meta.sha256)
            .map_err(|_| Erro::AssinaturaInvalida)?
            .confere(bytes)
    {
        return Err(Erro::AssinaturaInvalida);
    }
    let key = VerifyingKey::from_bytes(chave).map_err(|_| Erro::AssinaturaInvalida)?;
    let assinatura = STANDARD
        .decode(&meta.assinatura)
        .map_err(|_| Erro::AssinaturaInvalida)?;
    let sig = Signature::from_slice(&assinatura).map_err(|_| Erro::AssinaturaInvalida)?;
    key.verify_strict(bytes, &sig)
        .map_err(|_| Erro::AssinaturaInvalida)
}
pub async fn autoatualizar(cliente: &Cliente, atual: &Versao) -> Result<Option<Pronto>> {
    // O executável publicado é só para Windows: fora dele a atualização é manual.
    if !cfg!(windows) {
        return Ok(None);
    }
    let key = option_env!("RL_UPDATE_PUBKEY").ok_or(Erro::AtualizacaoNaoConfigurada)?;
    let chave: [u8; 32] = STANDARD
        .decode(key)
        .map_err(|_| Erro::AtualizacaoNaoConfigurada)?
        .try_into()
        .map_err(|_| Erro::AtualizacaoNaoConfigurada)?;
    autoatualizar_com(cliente, atual, chave).await
}
pub async fn autoatualizar_com(
    cliente: &Cliente,
    atual: &Versao,
    chave: [u8; 32],
) -> Result<Option<Pronto>> {
    let meta: Ultimo = cliente.get_json("/rl/v1/launcher/ultimo", None).await?;
    let nova = Versao::parse(&meta.versao).map_err(|_| Erro::ConfiguracaoInvalida)?;
    if nova <= *atual {
        return Ok(None);
    }
    if meta.tamanho > 100 * 1024 * 1024 {
        return Err(Erro::AssinaturaInvalida);
    }
    let mut response = cliente
        .get_stream(&format!("/rl/v1/launcher/{nova}"), None)
        .await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(Erro::rede)? {
        if bytes.len() + chunk.len() > meta.tamanho as usize {
            return Err(Erro::AssinaturaInvalida);
        }
        bytes.extend_from_slice(&chunk);
    }
    verificar(&meta, &bytes, &chave)?;
    Ok(Some(Pronto { bytes, meta, chave }))
}
#[derive(Debug, PartialEq, Eq)]
pub enum Evento {
    AtualizacaoPronta { versao: u64 },
}
pub async fn vigiar_pack(
    cliente: Cliente,
    mut versao: u64,
    eventos: tokio::sync::mpsc::Sender<Evento>,
) -> Result<()> {
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(600));
    timer.tick().await;
    loop {
        tokio::select! {
            _ = eventos.closed() => return Ok(()),
            _ = timer.tick() => {}
        }
        let m: Manifesto = tokio::select! {
            _ = eventos.closed() => return Ok(()),
            resposta = cliente.get_json("/rl/v1/manifesto", None) => match resposta {
                Ok(m) => m,
                // Uma indisponibilidade temporaria nao desativa a vigia.
                Err(_) => continue,
            }
        };
        if m.versao > versao {
            versao = m.versao;
            if eventos
                .send(Evento::AtualizacaoPronta { versao })
                .await
                .is_err()
            {
                return Ok(());
            }
        }
    }
}
