use crate::{
    baixar::{baixar_todos, Fallback, Hash, Item, Progresso},
    cliente::Cliente,
    contrato::{Arquivo, Manifesto, Origem},
    erro::{Erro, Result},
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use tokio::sync::mpsc::Sender;
pub struct Plano {
    pub baixar: Vec<Arquivo>,
    pub remover: Vec<PathBuf>,
}
pub struct Resumo {
    pub bytes: u64,
    pub baixados: usize,
    pub removidos: usize,
}
pub fn caminho(raiz: &Path, relativo: &str) -> Result<PathBuf> {
    if relativo.is_empty() || relativo.contains('\\') {
        return Err(Erro::CaminhoInvalido);
    }
    let mut path = raiz.to_path_buf();
    if raiz.exists() && std::fs::symlink_metadata(raiz)?.file_type().is_symlink() {
        return Err(Erro::CaminhoInvalido);
    }
    for parte in relativo.split('/') {
        let nome = parte
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if parte.is_empty()
            || parte == "."
            || parte == ".."
            || parte.ends_with([' ', '.'])
            || parte.chars().any(|c| c < ' ' || "<>:\"|?*".contains(c))
            || matches!(nome.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (nome.len() == 4
                && (nome.starts_with("COM") || nome.starts_with("LPT"))
                && nome.as_bytes()[3].is_ascii_digit())
        {
            return Err(Erro::CaminhoInvalido);
        }
        path.push(parte);
        match std::fs::symlink_metadata(&path) {
            Ok(m) if m.file_type().is_symlink() => return Err(Erro::CaminhoInvalido),
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(path)
}
pub fn plano(m: &Manifesto, raiz: &Path) -> Result<Plano> {
    let mut baixar = Vec::new();
    let mut remover = Vec::new();
    let mut conhecidos = HashSet::new();
    for a in &m.arquivos {
        let path = caminho(raiz, &a.caminho)?;
        if !conhecidos.insert(a.caminho.to_lowercase()) {
            return Err(Erro::CaminhoInvalido);
        }
        let hash = Hash::sha256(&a.sha256)?;
        if path.exists() {
            if !path.is_file() {
                return Err(Erro::CaminhoInvalido);
            }
            if a.uma_vez || hash.arquivo(&path)? {
                continue;
            }
        }
        baixar.push(a.clone());
    }
    for pasta in &m.pastas_gerenciadas {
        let path = caminho(raiz, pasta)?;
        if path.exists() {
            listar(raiz, &path, &conhecidos, &mut remover)?;
        }
    }
    remover.sort();
    remover.dedup();
    Ok(Plano { baixar, remover })
}
fn listar(
    raiz: &Path,
    dir: &Path,
    conhecidos: &HashSet<String>,
    remover: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let tipo = entry.file_type()?;
        if tipo.is_symlink() {
            return Err(Erro::CaminhoInvalido);
        }
        if tipo.is_dir() {
            listar(raiz, &path, conhecidos, remover)?;
        } else if tipo.is_file() {
            let relativo = path
                .strip_prefix(raiz)
                .map_err(|_| Erro::CaminhoInvalido)?
                .to_str()
                .ok_or(Erro::CaminhoInvalido)?
                .replace('\\', "/")
                .to_lowercase();
            if !conhecidos.contains(&relativo) && !relativo.ends_with(".part") {
                remover.push(path);
            }
        }
    }
    Ok(())
}
pub async fn sincronizar(
    m: &Manifesto,
    raiz: &Path,
    cliente: &Cliente,
    prog: Sender<Progresso>,
) -> Result<Resumo> {
    let p = plano(m, raiz)?;
    let mut itens = Vec::new();
    let mut bytes = 0;
    for a in &p.baixar {
        let remoto = format!("/rl/v1/arquivo/{}", a.sha256);
        bytes += a.tamanho;
        itens.push(Item {
            url: if a.origem == Origem::Servidor {
                remoto.clone()
            } else {
                a.url.clone()
            },
            destino: caminho(raiz, &a.caminho)?,
            sha: Hash::sha256(&a.sha256)?,
            tamanho: Some(a.tamanho),
            fallback: Some(Fallback {
                cliente: cliente.clone(),
                caminho: remoto,
            }),
        });
    }
    baixar_todos(itens, prog, 2).await?;
    for path in &p.remover {
        std::fs::remove_file(path)?;
    }
    Ok(Resumo {
        bytes,
        baixados: p.baixar.len(),
        removidos: p.remover.len(),
    })
}
