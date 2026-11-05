use crate::{
    cliente::Cliente,
    contrato::{Conta, LoginResp},
    erro::{Erro, Result},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Sessao {
    pub access: String,
    pub refresh: String,
    pub expira_em: u64,
    pub conta: Conta,
}
impl Sessao {
    pub fn de_resposta(r: LoginResp) -> Result<Self> {
        if r.conta.perfil.is_none() {
            return Err(Erro::SemAprovacao {
                discord_id: r.conta.discord_id,
            });
        }
        Ok(Self {
            access: r.access,
            refresh: r.refresh,
            expira_em: r.expira_em,
            conta: r.conta,
        })
    }
    pub async fn garantir(&mut self, cliente: &Cliente) -> Result<()> {
        self.garantir_com(cliente, &CofreSo).await
    }
    pub async fn garantir_com(&mut self, cliente: &Cliente, cofre: &dyn Cofre) -> Result<()> {
        let agora = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Erro::ConfiguracaoInvalida)?
            .as_millis() as u64;
        if self.expira_em > agora + 600_000 {
            return Ok(());
        }
        let resposta: Result<LoginResp> = cliente
            .post_json(
                "/rl/v1/refresh",
                &serde_json::json!({"refresh":self.refresh}),
            )
            .await;
        match resposta {
            Ok(resp) => {
                *self = Self::de_resposta(resp)?;
                cofre.guardar(self)
            }
            Err(Erro::Http(401 | 403)) => {
                self.access.clear();
                self.refresh.clear();
                self.expira_em = 0;
                cofre.apagar()?;
                Err(Erro::ContaRevogada)
            }
            Err(e) => Err(e),
        }
    }
}
pub trait Cofre: Send + Sync {
    fn guardar(&self, sessao: &Sessao) -> Result<()>;
    fn apagar(&self) -> Result<()>;
}
pub struct CofreSo;
#[cfg(windows)]
fn entrada() -> Result<keyring_core::Entry> {
    let store = windows_native_keyring_store::Store::new().map_err(|_| Erro::Cofre)?;
    store
        .build("Reliquias", "sessao", None)
        .map_err(|_| Erro::Cofre)
}
/// Fora do Windows a sessão fica em arquivo só do usuário (modo 0600).
#[cfg(not(windows))]
fn arquivo_sessao() -> Result<std::path::PathBuf> {
    Ok(crate::config::pasta_base(false)?.join("Reliquias-Launcher/sessao.json"))
}
#[cfg(windows)]
impl Cofre for CofreSo {
    fn guardar(&self, s: &Sessao) -> Result<()> {
        entrada()?
            .set_password(&serde_json::to_string(s)?)
            .map_err(|_| Erro::Cofre)
    }
    fn apagar(&self) -> Result<()> {
        match entrada()?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(_) => Err(Erro::Cofre),
        }
    }
}
#[cfg(not(windows))]
impl Cofre for CofreSo {
    fn guardar(&self, s: &Sessao) -> Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let p = arquivo_sessao()?;
        let dir = p.parent().ok_or(Erro::Cofre)?;
        std::fs::create_dir_all(dir).map_err(|_| Erro::Cofre)?;
        let tmp = p.with_extension("tmp");
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|_| Erro::Cofre)?;
        f.write_all(&serde_json::to_vec(s)?).map_err(|_| Erro::Cofre)?;
        f.sync_all().map_err(|_| Erro::Cofre)?;
        std::fs::rename(&tmp, &p).map_err(|_| Erro::Cofre)
    }
    fn apagar(&self) -> Result<()> {
        match std::fs::remove_file(arquivo_sessao()?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(Erro::Cofre),
        }
    }
}
pub fn guardar(s: &Sessao) -> Result<()> {
    CofreSo.guardar(s)
}
#[cfg(windows)]
pub fn carregar() -> Option<Sessao> {
    serde_json::from_str(&entrada().ok()?.get_password().ok()?).ok()
}
#[cfg(not(windows))]
pub fn carregar() -> Option<Sessao> {
    serde_json::from_slice(&std::fs::read(arquivo_sessao().ok()?).ok()?).ok()
}

#[cfg(windows)]
use keyring_core::api::CredentialStoreApi;
