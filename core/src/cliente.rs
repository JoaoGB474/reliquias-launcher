use crate::{
    erro::{Erro, Result},
    tls::Fixado,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{sync::Arc, time::Duration};
#[derive(Clone)]
pub struct Cliente {
    base: reqwest::Url,
    http: reqwest::Client,
    stream: reqwest::Client,
}
impl Cliente {
    pub async fn novidades(&self) -> Result<crate::contrato::Novidades> {
        self.get_json("/rl/v1/novidades", None).await
    }

    pub fn desenvolvimento() -> Result<Self> {
        Self::novo(env!("RL_SERVIDOR"), crate::SPKI)
    }
    pub fn novo(host: &str, pin: [u8; 32]) -> Result<Self> {
        let base =
            reqwest::Url::parse(&format!("https://{host}/")).map_err(|_| Erro::EnderecoInvalido)?;
        if base.host_str().is_none()
            || base.path() != "/"
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(Erro::EnderecoInvalido);
        }
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| Erro::ServidorDesconhecido)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Fixado(pin)))
        .with_no_client_auth();
        let builder = || {
            reqwest::Client::builder()
                .use_preconfigured_tls(config.clone())
                .https_only(true)
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(10))
                .read_timeout(Duration::from_secs(30))
        };
        Ok(Self {
            base,
            http: builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(Erro::rede)?,
            stream: builder().build().map_err(Erro::rede)?,
        })
    }
    pub fn url(&self, caminho: &str) -> Result<reqwest::Url> {
        if !caminho.starts_with('/') || caminho.starts_with("//") || caminho.contains('\\') {
            return Err(Erro::EnderecoInvalido);
        }
        let url = self
            .base
            .join(caminho)
            .map_err(|_| Erro::EnderecoInvalido)?;
        if url.origin() != self.base.origin() {
            return Err(Erro::EnderecoInvalido);
        }
        Ok(url)
    }
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        caminho: &str,
        bearer: Option<&str>,
    ) -> Result<T> {
        let mut req = self.http.get(self.url(caminho)?);
        if let Some(token) = bearer {
            req = req.bearer_auth(token);
        }
        req.send()
            .await
            .map_err(Erro::rede)?
            .error_for_status()
            .map_err(Erro::rede)?
            .json()
            .await
            .map_err(Erro::rede)
    }
    pub async fn post_json<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        caminho: &str,
        corpo: &B,
    ) -> Result<T> {
        self.http
            .post(self.url(caminho)?)
            .json(corpo)
            .send()
            .await
            .map_err(Erro::rede)?
            .error_for_status()
            .map_err(Erro::rede)?
            .json()
            .await
            .map_err(Erro::rede)
    }
    pub async fn get_stream(&self, caminho: &str, range: Option<u64>) -> Result<reqwest::Response> {
        let mut req = self.stream.get(self.url(caminho)?);
        if let Some(inicio) = range {
            req = req.header(reqwest::header::RANGE, format!("bytes={inicio}-"));
        }
        req.send()
            .await
            .map_err(Erro::rede)?
            .error_for_status()
            .map_err(Erro::rede)
    }
}
impl Cliente {
    pub async fn encaminhar(
        &self,
        metodo: reqwest::Method,
        caminho: &str,
        corpo: Vec<u8>,
    ) -> Result<reqwest::Response> {
        self.http
            .request(metodo, self.url(caminho)?)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(corpo)
            .send()
            .await
            .map_err(Erro::rede)
    }
}
