use crate::{
    args::{montar_args, Contexto},
    baixar::Progresso,
    cliente::Cliente,
    contrato::Manifesto,
    erro::{Erro, Result},
    instancia::{Instancia, Trava},
    proxy::Proxy,
    sessao::Sessao,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    sync::{mpsc::Sender, Mutex},
};
pub struct Jogo {
    pub filho: tokio::process::Child,
    _proxy: Proxy,
    _trava: Trava,
    logs: Vec<tokio::task::JoinHandle<Result<()>>>,
}
impl Jogo {
    pub async fn esperar(mut self) -> Result<std::process::ExitStatus> {
        let status = self.filho.wait().await?;
        for job in self.logs.drain(..) {
            job.await.map_err(|_| Erro::Instalacao)??;
        }
        Ok(status)
    }
}
pub async fn preparar(
    m: &Manifesto,
    inst: &Instancia,
    cliente: &Cliente,
    sessao: Arc<Mutex<Sessao>>,
    memoria_mb: u32,
    prog: Sender<Progresso>,
) -> Result<(Contexto, crate::args::VersaoJson, Proxy)> {
    crate::pack::sincronizar(m, &inst.raiz, cliente, prog.clone()).await?;
    if let Some(servidor) = &m.servidor {
        crate::interface::garantir_servidor(inst, &servidor.nome, &servidor.endereco)?;
    }
    let instalado = crate::mojang::instalar(m, inst, prog).await?;
    let mut s = sessao.lock().await;
    s.garantir(cliente).await?;
    let perfil = s.conta.perfil.clone().ok_or_else(|| Erro::SemAprovacao {
        discord_id: s.conta.discord_id.clone(),
    })?;
    drop(s);
    let metadados: serde_json::Value = cliente.get_json("/ygg/", None).await?;
    let proxy = crate::proxy::iniciar(cliente.clone(), sessao).await?;
    let ctx = Contexto {
        java: instalado.java,
        raiz: inst.raiz.clone(),
        usuario: perfil.nick,
        uuid: perfil.id,
        access_falso: "x".into(),
        agent: instalado.agent,
        proxy_url: format!("http://127.0.0.1:{}/ygg", proxy.porta),
        memoria_mb,
        classpath: instalado.classpath,
        prefetched: STANDARD.encode(serde_json::to_vec(&metadados)?),
    };
    Ok((ctx, instalado.versao, proxy))
}
pub async fn lancar(
    ctx: &Contexto,
    versao: &crate::args::VersaoJson,
    proxy: Proxy,
    trava: Trava,
    sessao: Arc<Mutex<Sessao>>,
) -> Result<Jogo> {
    let logs = ctx.raiz.join("logs");
    tokio::fs::create_dir_all(&logs).await?;
    let file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(logs.join("launcher.log"))
        .await?;
    let file = Arc::new(Mutex::new(file));
    let mut comando = tokio::process::Command::new(&ctx.java);
    comando
        .args(montar_args(versao, ctx))
        .current_dir(&ctx.raiz)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // java.exe é programa de console: sem esta flag o Windows abre um terminal junto com o jogo.
    #[cfg(windows)]
    comando.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut filho = comando.spawn()?;
    let stdout = filho.stdout.take().ok_or(Erro::Instalacao)?;
    let stderr = filho.stderr.take().ok_or(Erro::Instalacao)?;
    let a = tokio::spawn(capturar(stdout, file.clone(), sessao.clone()));
    let b = tokio::spawn(capturar(stderr, file, sessao));
    Ok(Jogo {
        filho,
        _proxy: proxy,
        _trava: trava,
        logs: vec![a, b],
    })
}
async fn capturar(
    reader: impl AsyncRead + Unpin,
    file: Arc<Mutex<tokio::fs::File>>,
    sessao: Arc<Mutex<Sessao>>,
) -> Result<()> {
    let mut lines = BufReader::new(reader).lines();
    while let Some(mut line) = lines.next_line().await? {
        let s = sessao.lock().await;
        for secret in [&s.access, &s.refresh] {
            if !secret.is_empty() {
                line = line.replace(secret, "[removido]");
            }
        }
        drop(s);
        let mut f = file.lock().await;
        f.write_all(line.as_bytes()).await?;
        f.write_all(b"\n").await?;
    }
    Ok(())
}
