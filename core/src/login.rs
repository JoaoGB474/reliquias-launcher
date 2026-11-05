use crate::{
    cliente::Cliente,
    erro::{Erro, Result},
    sessao::Sessao,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
fn aleatorio() -> String {
    let mut bytes = [0; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
pub async fn ouvir(portas: &[u16]) -> Result<TcpListener> {
    for p in portas {
        match TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, *p)).await {
            Ok(l) => return Ok(l),
            Err(e)
                if e.kind() == std::io::ErrorKind::AddrInUse
                    || e.kind() == std::io::ErrorKind::PermissionDenied => {}
            Err(e) => return Err(e.into()),
        }
    }
    Err(Erro::PortasOcupadas)
}
pub fn validar_callback(target: &str, state: &str) -> Result<String> {
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| Erro::CallbackInvalido)?;
    if url.path() != "/callback" {
        return Err(Erro::CallbackInvalido);
    }
    let mut codigo = None;
    let mut recebido = None;
    for (k, v) in url.query_pairs() {
        let slot = match k.as_ref() {
            "code" => &mut codigo,
            "state" => &mut recebido,
            _ => continue,
        };
        if slot.replace(v.into_owned()).is_some() {
            return Err(Erro::CallbackInvalido);
        }
    }
    if recebido.as_deref() != Some(state) {
        return Err(Erro::CallbackInvalido);
    }
    codigo
        .filter(|s| !s.is_empty())
        .ok_or(Erro::CallbackInvalido)
}
pub async fn entrar(cliente: &Cliente, abrir: impl Fn(&str)) -> Result<Sessao> {
    let id = option_env!("RL_DISCORD_CLIENT_ID")
        .filter(|s| !s.is_empty())
        .ok_or(Erro::DiscordNaoConfigurado)?;
    let listener = ouvir(&[49152, 49153, 49154]).await?;
    let redirect = format!(
        "http://127.0.0.1:{}/callback",
        listener.local_addr()?.port()
    );
    let verifier = aleatorio();
    let state = aleatorio();
    let mut url = reqwest::Url::parse("https://discord.com/oauth2/authorize")
        .map_err(|_| Erro::EnderecoInvalido)?;
    url.query_pairs_mut().extend_pairs([
        ("client_id", id),
        ("response_type", "code"),
        ("scope", "identify"),
        ("redirect_uri", redirect.as_str()),
        ("code_challenge_method", "S256"),
        ("code_challenge", challenge(&verifier).as_str()),
        ("state", state.as_str()),
    ]);
    abrir(url.as_str());
    let code=tokio::time::timeout(std::time::Duration::from_secs(180),async {
        loop {let(mut socket,_)=listener.accept().await?;let request=tokio::time::timeout(std::time::Duration::from_secs(5),async{let mut bytes=Vec::new();loop{let mut b=[0;1024];let n=socket.read(&mut b).await?;if n==0||bytes.len()+n>8192{return Err(Erro::CallbackInvalido);}bytes.extend_from_slice(&b[..n]);if bytes.windows(4).any(|w|w==b"\r\n\r\n"){break;}}Ok(bytes)}).await;
            let Ok(Ok(bytes))=request else{continue;};let text=String::from_utf8_lossy(&bytes);let mut parts=text.split_whitespace();if parts.next()!=Some("GET"){continue;}let Some(target)=parts.next()else{continue;};let resultado=validar_callback(target,&state);
            let status=if resultado.is_ok(){"200 OK"}else{"400 Bad Request"};let body="Pode fechar esta aba.";socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await?;
            if let Ok(code)=resultado{return Ok::<_,Erro>(code);}
        }
    }).await.map_err(|_|Erro::LoginExpirado)??;
    let resp = cliente
        .post_json(
            "/rl/v1/login",
            &serde_json::json!({"code":code,"verifier":verifier,"redirect_uri":redirect}),
        )
        .await?;
    let sessao = Sessao::de_resposta(resp)?;
    crate::sessao::guardar(&sessao)?;
    Ok(sessao)
}
