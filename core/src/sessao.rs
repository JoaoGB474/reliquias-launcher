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
fn entrada() -> Result<keyring_core::Entry> {
    #[cfg(windows)]
    {
        let store = windows_native_keyring_store::Store::new().map_err(|_| Erro::Cofre)?;
        store
            .build("Reliquias", "sessao", None)
            .map_err(|_| Erro::Cofre)
    }
    #[cfg(not(windows))]
    {
        Err(Erro::Cofre)
    }
}
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
pub fn guardar(s: &Sessao) -> Result<()> {
    CofreSo.guardar(s)
}
pub fn carregar() -> Option<Sessao> {
    serde_json::from_str(&entrada().ok()?.get_password().ok()?).ok()
}

#[cfg(windows)]
use keyring_core::api::CredentialStoreApi;
