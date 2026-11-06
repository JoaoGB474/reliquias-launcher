use crate::{
    baixar::{baixar_todos, Hash, Item, Progresso},
    erro::{Erro, Result},
    instancia::Instancia,
    pack::caminho,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::sync::mpsc::Sender;
pub const RUNTIMES:&str="https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Download {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}
#[derive(Deserialize, Serialize)]
pub struct Runtime {
    pub files: BTreeMap<String, Entrada>,
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Entrada {
    Directory,
    File {
        downloads: BTreeMap<String, Download>,
        #[serde(default)]
        executable: bool,
    },
    Link {
        target: String,
    },
}
pub fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .use_rustls_tls()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(Erro::rede)
}
pub async fn json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
    http()?
        .get(url)
        .send()
        .await
        .map_err(Erro::rede)?
        .error_for_status()
        .map_err(Erro::rede)?
        .json()
        .await
        .map_err(Erro::rede)
}
pub async fn instalar_manifesto(m: &Runtime, raiz: &Path, prog: Sender<Progresso>) -> Result<()> {
    let mut itens = Vec::new();
    let mut links: Vec<(PathBuf, String)> = Vec::new();
    for (nome, entrada) in &m.files {
        let destino = caminho(raiz, nome)?;
        match entrada {
            Entrada::Directory => std::fs::create_dir_all(destino)?,
            Entrada::File { downloads, .. } => {
                let raw = downloads.get("raw").ok_or(Erro::ConfiguracaoInvalida)?;
                let sha = Hash::sha1(&raw.sha1)?;
                if destino.exists() && sha.arquivo(&destino)? {
                    continue;
                }
                itens.push(Item {
                    url: raw.url.clone(),
                    destino,
                    sha,
                    tamanho: Some(raw.size),
                    fallback: None,
                });
            }
            Entrada::Link { target } => links.push((destino, target.clone())),
        }
    }
    baixar_todos(itens, prog, 8).await?;
    #[cfg(not(unix))]
    if !links.is_empty() {
        return Err(Erro::ConfiguracaoInvalida);
    }
    #[cfg(unix)]
    {
        for (destino, alvo) in &links {
            criar_link(raiz, destino, alvo)?;
        }
        use std::os::unix::fs::PermissionsExt;
        for (nome, entrada) in &m.files {
            if matches!(
                entrada,
                Entrada::File {
                    executable: true,
                    ..
                }
            ) {
                std::fs::set_permissions(
                    caminho(raiz, nome)?,
                    std::fs::Permissions::from_mode(0o755),
                )?;
            }
        }
    }
    Ok(())
}
/// Cria o link simbólico do runtime, recusando alvo absoluto ou que saia da pasta do runtime.
#[cfg(unix)]
fn criar_link(raiz: &Path, destino: &Path, alvo: &str) -> Result<()> {
    use std::path::Component;
    let mut atual: Vec<_> = destino
        .parent()
        .and_then(|p| p.strip_prefix(raiz).ok())
        .ok_or(Erro::ConfiguracaoInvalida)?
        .components()
        .collect();
    for c in Path::new(alvo).components() {
        match c {
            Component::Normal(_) => atual.push(c),
            Component::CurDir => {}
            Component::ParentDir => {
                atual.pop().ok_or(Erro::ConfiguracaoInvalida)?;
            }
            _ => return Err(Erro::ConfiguracaoInvalida),
        }
    }
    if let Some(pai) = destino.parent() {
        std::fs::create_dir_all(pai)?;
    }
    if let Ok(meta) = std::fs::symlink_metadata(destino) {
        if meta.file_type().is_symlink() && std::fs::read_link(destino).ok().as_deref() == Some(Path::new(alvo)) {
            return Ok(());
        }
        if meta.is_dir() {
            return Err(Erro::ConfiguracaoInvalida);
        }
        std::fs::remove_file(destino)?;
    }
    std::os::unix::fs::symlink(alvo, destino)?;
    Ok(())
}
pub async fn garantir(inst: &Instancia, versao: u8, prog: Sender<Progresso>) -> Result<PathBuf> {
    if versao != 21 {
        return Err(Erro::ConfiguracaoInvalida);
    }
    let raiz = inst.raiz.join("runtime/java-runtime-delta");
    std::fs::create_dir_all(&raiz)?;
    let cache = raiz.join("manifest.json");
    let m: Runtime = if cache.exists() {
        serde_json::from_slice(&std::fs::read(&cache)?)?
    } else {
        let all: serde_json::Value = json(RUNTIMES).await?;
        let platform = if cfg!(target_os = "windows") {
            "windows-x64"
        } else {
            "linux"
        };
        let download: Download =
            serde_json::from_value(all[platform]["java-runtime-delta"][0]["manifest"].clone())?;
        baixar_todos(
            vec![Item {
                url: download.url,
                destino: cache.clone(),
                sha: Hash::sha1(&download.sha1)?,
                tamanho: Some(download.size),
                fallback: None,
            }],
            prog.clone(),
            1,
        )
        .await?;
        serde_json::from_slice(&std::fs::read(&cache)?)?
    };
    instalar_manifesto(&m, &raiz, prog).await?;
    let java = raiz.join(if cfg!(windows) {
        "bin/java.exe"
    } else {
        "bin/java"
    });
    if !java.is_file() {
        return Err(Erro::ConfiguracaoInvalida);
    }
    Ok(java)
}
