use crate::{
    cliente::Cliente,
    erro::{Erro, Result},
    sessao::{Cofre, CofreSo, Sessao},
};
use http_body_util::{BodyExt, Full, Limited};
use hyper::{body::Bytes, Request, Response, StatusCode};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
pub struct Proxy {
    pub porta: u16,
    tarefa: tokio::task::JoinHandle<()>,
}
impl Proxy {
    pub fn endereco(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.porta))
    }
    pub async fn parar(self) {
        self.tarefa.abort();
    }
}
impl Drop for Proxy {
    fn drop(&mut self) {
        self.tarefa.abort();
    }
}
fn resposta(status: StatusCode, body: impl Into<Bytes>) -> Response<Full<Bytes>> {
    let mut r = Response::new(Full::new(body.into()));
    *r.status_mut() = status;
    r
}
pub async fn iniciar(cliente: Cliente, sessao: Arc<Mutex<Sessao>>) -> Result<Proxy> {
    iniciar_com(cliente, sessao, Arc::new(CofreSo)).await
}
pub async fn iniciar_com(
    cliente: Cliente,
    sessao: Arc<Mutex<Sessao>>,
    cofre: Arc<dyn Cofre>,
) -> Result<Proxy> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let porta = listener.local_addr()?.port();
    let tarefa = tokio::spawn(async move {
        let mut conexoes = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                result=listener.accept()=>{let Ok((socket,_))=result else{break;};let cliente=cliente.clone();let sessao=sessao.clone();let cofre=cofre.clone();conexoes.spawn(async move{
                    let service=hyper::service::service_fn(move |req|{let c=cliente.clone();let s=sessao.clone();let cofre=cofre.clone();async move{let r=atender(req,c,s,cofre).await.unwrap_or_else(|_|resposta(StatusCode::BAD_GATEWAY,"falha na autenticação"));Ok::<_,Infallible>(r)}});
                    let conn=hyper::server::conn::http1::Builder::new().serve_connection(hyper_util::rt::TokioIo::new(socket),service);let _=tokio::time::timeout(std::time::Duration::from_secs(65),conn).await;
                });},_=conexoes.join_next(),if !conexoes.is_empty()=>{}
            }
        }
    });
    Ok(Proxy { porta, tarefa })
}
async fn atender(
    req: Request<hyper::body::Incoming>,
    cliente: Cliente,
    sessao: Arc<Mutex<Sessao>>,
    cofre: Arc<dyn Cofre>,
) -> Result<Response<Full<Bytes>>> {
    let path = req.uri().path();
    if !path.starts_with("/ygg/")
        || path.contains('%')
        || path.split('/').any(|p| p == ".." || p == ".")
        || req.uri().scheme().is_some()
    {
        return Ok(resposta(StatusCode::NOT_FOUND, Bytes::new()));
    }
    if req.headers().contains_key("origin") {
        return Ok(resposta(StatusCode::FORBIDDEN, Bytes::new()));
    }
    let join =
        path == "/ygg/sessionserver/session/minecraft/join" && req.method() == hyper::Method::POST;
    let caminho = req
        .uri()
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or(path)
        .to_owned();
    let metodo = req.method().clone();
    let bytes = Limited::new(req.into_body(), 1024 * 1024)
        .collect()
        .await
        .map_err(|_| Erro::CallbackInvalido)?
        .to_bytes();
    let mut corpo = bytes.to_vec();
    let mut usado = String::new();
    if join {
        let mut s = sessao.lock().await;
        s.garantir_com(&cliente, cofre.as_ref()).await?;
        usado = s.access.clone();
        let mut json: serde_json::Value = serde_json::from_slice(&corpo)?;
        let obj = json.as_object_mut().ok_or(Erro::CallbackInvalido)?;
        obj.insert(
            "accessToken".into(),
            serde_json::Value::String(usado.clone()),
        );
        corpo = serde_json::to_vec(&json)?;
    }
    let mut upstream = cliente
        .encaminhar(metodo.clone(), &caminho, corpo.clone())
        .await?;
    if upstream.status() == reqwest::StatusCode::UNAUTHORIZED && join {
        let mut s = sessao.lock().await;
        if s.access == usado {
            s.expira_em = 0;
            s.garantir_com(&cliente, cofre.as_ref()).await?;
        }
        let mut json: serde_json::Value = serde_json::from_slice(&corpo)?;
        json["accessToken"] = serde_json::Value::String(s.access.clone());
        drop(s);
        upstream = cliente
            .encaminhar(metodo, &caminho, serde_json::to_vec(&json)?)
            .await?;
    }
    let status = upstream.status();
    let tipo = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .cloned();
    let mut body = Vec::new();
    while let Some(b) = upstream.chunk().await.map_err(Erro::rede)? {
        if body.len() + b.len() > 4 * 1024 * 1024 {
            return Err(Erro::ConfiguracaoInvalida);
        }
        body.extend_from_slice(&b);
    }
    let mut r = resposta(status, body);
    if let Some(tipo) = tipo {
        r.headers_mut().insert(hyper::header::CONTENT_TYPE, tipo);
    }
    Ok(r)
}
