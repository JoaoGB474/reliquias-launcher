#[derive(Debug, thiserror::Error)]
pub enum Erro {
    #[error("O jogo fechou com erro (código {})", codigo.map_or_else(|| "desconhecido".into(), |c| c.to_string()))]
    JogoSaiu { codigo: Option<i32> },
    #[error("assinatura inválida")]
    AssinaturaInvalida,
    #[error("chave pública de atualização não configurada no build")]
    AtualizacaoNaoConfigurada,
    #[error("conta revogada; entre novamente")]
    ContaRevogada,
    #[error("aguardando aprovação: {discord_id}")]
    SemAprovacao { discord_id: String },
    #[error("portas de login ocupadas")]
    PortasOcupadas,
    #[error("retorno de login inválido")]
    CallbackInvalido,
    #[error("login expirado")]
    LoginExpirado,
    #[error("Discord não configurado no build")]
    DiscordNaoConfigurado,
    #[error("cofre do sistema indisponível")]
    Cofre,
    #[error("falha na instalação do jogo")]
    Instalacao,
    #[error("launcher já aberto")]
    JaAberto,
    #[error("hash inválido")]
    HashInvalido,
    #[error("espaço insuficiente no disco")]
    SemEspaco,
    #[error("caminho inválido")]
    CaminhoInvalido,
    #[error("configuração inválida")]
    ConfiguracaoInvalida,
    #[error("servidor não reconhecido")]
    ServidorDesconhecido,
    #[error("endereço inválido")]
    EnderecoInvalido,
    #[error("falha de rede")]
    Rede,
    #[error("servidor respondeu com erro {0}")]
    Http(u16),
    #[error("falha ao acessar arquivo")]
    Io(#[from] std::io::Error),
    #[error("resposta inválida do servidor")]
    Json(#[from] serde_json::Error),
}
impl Erro {
    pub fn rede(err: reqwest::Error) -> Self {
        use std::error::Error;
        let mut causa: Option<&(dyn Error + 'static)> = Some(&err);
        while let Some(e) = causa {
            if e.to_string().contains("servidor não reconhecido") {
                return Self::ServidorDesconhecido;
            }
            causa = e.source();
        }
        match err.status() {
            Some(status) => Self::Http(status.as_u16()),
            None => Self::Rede,
        }
    }
}
pub type Result<T> = std::result::Result<T, Erro>;
