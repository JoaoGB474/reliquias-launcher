use crate::{
    cliente::Cliente,
    erro::{Erro, Result},
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::AsyncWriteExt,
    sync::{mpsc::Sender, Mutex},
};
#[derive(Clone, Debug)]
pub enum Hash {
    Sha256([u8; 32]),
    Sha1([u8; 20]),
}
impl Hash {
    pub fn sha256(hex: &str) -> Result<Self> {
        Ok(Self::Sha256(decodificar(hex)?))
    }
    pub fn sha1(hex: &str) -> Result<Self> {
        Ok(Self::Sha1(decodificar(hex)?))
    }
    pub fn confere(&self, bytes: &[u8]) -> bool {
        match self {
            Self::Sha256(h) => Sha256::digest(bytes).as_slice() == h,
            Self::Sha1(h) => sha1::Sha1::digest(bytes).as_slice() == h,
        }
    }
    pub fn arquivo(&self, path: &Path) -> Result<bool> {
        use std::io::Read;
        let mut file = std::fs::File::open(path)?;
        let mut sha256 = Sha256::new();
        let mut sha1 = sha1::Sha1::new();
        let mut buf = [0; 65536];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            match self {
                Self::Sha256(_) => sha256.update(&buf[..n]),
                Self::Sha1(_) => sha1.update(&buf[..n]),
            }
        }
        Ok(match self {
            Self::Sha256(h) => sha256.finalize().as_slice() == h,
            Self::Sha1(h) => sha1.finalize().as_slice() == h,
        })
    }
}
fn decodificar<const N: usize>(hex: &str) -> Result<[u8; N]> {
    if hex.len() != N * 2 || !hex.is_ascii() {
        return Err(Erro::HashInvalido);
    }
    let mut out = [0; N];
    for (i, v) in out.iter_mut().enumerate() {
        *v = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| Erro::HashInvalido)?;
    }
    Ok(out)
}
#[derive(Clone)]
pub struct Fallback {
    pub cliente: Cliente,
    pub caminho: String,
}
#[derive(Clone)]
pub struct Item {
    pub url: String,
    pub destino: PathBuf,
    pub sha: Hash,
    pub tamanho: Option<u64>,
    pub fallback: Option<Fallback>,
}
#[derive(Clone, Debug, Default)]
pub struct Progresso {
    pub bytes: u64,
    pub concluidos: usize,
    pub total: usize,
}
fn io(err: std::io::Error) -> Erro {
    if matches!(err.raw_os_error(), Some(28 | 112 | 39)) {
        Erro::SemEspaco
    } else {
        Erro::Io(err)
    }
}
pub fn parcial(path: &Path) -> PathBuf {
    let mut nome = path.as_os_str().to_os_string();
    nome.push(".part");
    PathBuf::from(nome)
}
async fn baixar(
    item: Item,
    http: reqwest::Client,
    prog: Sender<Progresso>,
    estado: Arc<Mutex<Progresso>>,
) -> Result<()> {
    if let Some(parent) = item.destino.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(io)?;
    }
    let part = parcial(&item.destino);
    for path in [&item.destino, &part] {
        match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_symlink() => return Err(Erro::CaminhoInvalido),
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(io(e)),
        }
    }
    for tentativa in 0..2 {
        let resultado = transferir(&item, &part, &http, &prog, &estado).await;
        match resultado {
            Ok(()) => {
                tokio::fs::rename(&part, &item.destino).await.map_err(io)?;
                let mut p = estado.lock().await;
                p.concluidos += 1;
                let _ = prog.try_send(p.clone());
                return Ok(());
            }
            Err(e) => {
                if matches!(e, Erro::HashInvalido) {
                    match tokio::fs::remove_file(&part).await {
                        Ok(()) => (),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (),
                        Err(err) => return Err(io(err)),
                    }
                }
                if tentativa == 1
                    || matches!(
                        e,
                        Erro::SemEspaco
                            | Erro::ServidorDesconhecido
                            | Erro::Io(_)
                            | Erro::EnderecoInvalido
                    )
                {
                    return Err(e);
                }
            }
        }
    }
    Err(Erro::HashInvalido)
}
async fn transferir(
    item: &Item,
    part: &Path,
    http: &reqwest::Client,
    prog: &Sender<Progresso>,
    estado: &Mutex<Progresso>,
) -> Result<()> {
    let inicio = match tokio::fs::metadata(part).await {
        Ok(m) => m.len(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => return Err(io(e)),
    };
    let range = if inicio > 0 { Some(inicio) } else { None };
    let mut resposta = if item.url.starts_with('/') {
        let f = item.fallback.as_ref().ok_or(Erro::EnderecoInvalido)?;
        f.cliente.get_stream(&item.url, range).await?
    } else {
        let url = reqwest::Url::parse(&item.url).map_err(|_| Erro::EnderecoInvalido)?;
        // HTTP é permitido exclusivamente em loopback para testes locais.
        if url.scheme() != "https"
            && !(url.scheme() == "http" && url.host_str() == Some("127.0.0.1"))
        {
            return Err(Erro::EnderecoInvalido);
        }
        let mut req = http.get(url);
        if let Some(n) = range {
            req = req.header(reqwest::header::RANGE, format!("bytes={n}-"));
        }
        match req.send().await.and_then(|r| r.error_for_status()) {
            Ok(r) => r,
            Err(e) => {
                let elegivel = e.is_timeout()
                    || e.status()
                        .is_some_and(|s| s.is_client_error() || s.is_server_error());
                if !elegivel {
                    return Err(Erro::rede(e));
                }
                let f = item.fallback.as_ref().ok_or_else(|| Erro::rede(e))?;
                f.cliente.get_stream(&f.caminho, range).await?
            }
        }
    };
    let retomar = resposta.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    if retomar {
        let value = resposta
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .ok_or(Erro::HashInvalido)?;
        if !value.starts_with(&format!("bytes {inicio}-")) {
            return Err(Erro::HashInvalido);
        }
    } else if resposta.status() != reqwest::StatusCode::OK {
        return Err(Erro::Http(resposta.status().as_u16()));
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(!retomar)
        .append(retomar)
        .open(part)
        .await
        .map_err(io)?;
    let mut tamanho = if retomar { inicio } else { 0 };
    while let Some(chunk) = resposta.chunk().await.map_err(Erro::rede)? {
        tamanho += chunk.len() as u64;
        if item.tamanho.is_some_and(|n| tamanho > n) {
            return Err(Erro::HashInvalido);
        }
        file.write_all(&chunk).await.map_err(io)?;
        let mut p = estado.lock().await;
        p.bytes += chunk.len() as u64;
        let _ = prog.try_send(p.clone());
    }
    file.flush().await.map_err(io)?;
    file.sync_all().await.map_err(io)?;
    drop(file);
    if item.tamanho.is_some_and(|n| tamanho != n) || !item.sha.arquivo(part)? {
        return Err(Erro::HashInvalido);
    }
    Ok(())
}
pub async fn baixar_todos(
    itens: Vec<Item>,
    prog: Sender<Progresso>,
    paralelo: usize,
) -> Result<()> {
    if paralelo == 0 {
        return Err(Erro::ConfiguracaoInvalida);
    }
    // O índice de assets da Mojang repete o mesmo hash com nomes lógicos diferentes: mesmo destino
    // e mesmo hash é o mesmo arquivo (baixa uma vez). Mesmo destino com hash diferente é erro.
    let mut vistos: std::collections::HashMap<PathBuf, String> = std::collections::HashMap::new();
    let mut unicos = Vec::with_capacity(itens.len());
    for item in itens {
        let chave = format!("{:?}", item.sha);
        match vistos.get(&item.destino) {
            Some(c) if *c == chave => continue,
            Some(_) => return Err(Erro::CaminhoInvalido),
            None => {
                vistos.insert(item.destino.clone(), chave);
                unicos.push(item);
            }
        }
    }
    let itens = unicos;
    let http = reqwest::Client::builder()
        .use_rustls_tls()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(Erro::rede)?;
    let estado = Arc::new(Mutex::new(Progresso {
        total: itens.len(),
        ..Default::default()
    }));
    let sem = Arc::new(tokio::sync::Semaphore::new(paralelo));
    let mut jobs = tokio::task::JoinSet::new();
    for item in itens {
        let sem = sem.clone();
        let http = http.clone();
        let prog = prog.clone();
        let estado = estado.clone();
        jobs.spawn(async move {
            let _permit = sem
                .acquire_owned()
                .await
                .map_err(|_| Erro::ConfiguracaoInvalida)?;
            baixar(item, http, prog, estado).await
        });
    }
    while let Some(result) = jobs.join_next().await {
        result.map_err(|_| Erro::ConfiguracaoInvalida)??;
    }
    Ok(())
}
