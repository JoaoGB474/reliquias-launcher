use crate::{
    baixar::Progresso,
    erro::{Erro, Result},
};
use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
};
use tokio::sync::mpsc::Sender;
#[derive(Clone, Debug)]
pub struct Instancia {
    pub raiz: PathBuf,
}
pub struct Trava {
    _arquivo: File,
}
impl Instancia {
    pub fn padrao() -> Result<Self> {
        if let Some(raiz) = config::Config::carregar()?.pasta {
            return Ok(Self { raiz });
        }
        let base = std::env::var_os("APPDATA").ok_or(Erro::ConfiguracaoInvalida)?;
        Ok(Self {
            raiz: PathBuf::from(base).join("Reliquias"),
        })
    }
    pub fn trancar(&self) -> Result<Trava> {
        std::fs::create_dir_all(&self.raiz)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.raiz.join("launcher.lock"))?;
        fs2::FileExt::try_lock_exclusive(&file).map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(33) {
                Erro::JaAberto
            } else {
                Erro::Io(e)
            }
        })?;
        Ok(Trava { _arquivo: file })
    }
    pub async fn garantir_java(&self, versao: u8, prog: Sender<Progresso>) -> Result<PathBuf> {
        crate::java::garantir(self, versao, prog).await
    }
}
pub fn trancar() -> Result<Trava> {
    Instancia::padrao()?.trancar()
}

#[path = "config.rs"]
pub mod config;

use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

pub const ESPACO_MINIMO: u64 = 8 * 1024 * 1024 * 1024;
#[derive(Debug, PartialEq, Eq)]
pub enum AvisoPasta {
    Nenhum,
    NaoVazia,
    Sincronizada,
}
#[derive(Debug, thiserror::Error)]
pub enum ErroPasta {
    #[error("sem permissão para escrever na pasta")]
    SemPermissao,
    #[error("espaço insuficiente: {livre} bytes livres; necessários {preciso}")]
    SemEspaco { livre: u64, preciso: u64 },
    #[error("caminho inválido: escolha uma pasta com até 120 caracteres, fora da raiz do disco")]
    CaminhoInvalido,
    #[error("falha ao acessar a pasta")]
    Io(#[source] std::io::Error),
}
impl From<std::io::Error> for ErroPasta {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            Self::SemPermissao
        } else {
            Self::Io(e)
        }
    }
}
impl From<ErroPasta> for Erro {
    fn from(e: ErroPasta) -> Self {
        Self::Io(std::io::Error::other(e))
    }
}
fn conferir_espaco(livre: u64, preciso: u64) -> std::result::Result<(), ErroPasta> {
    if livre < preciso {
        Err(ErroPasta::SemEspaco { livre, preciso })
    } else {
        Ok(())
    }
}
pub fn validar_pasta(p: &Path) -> std::result::Result<AvisoPasta, ErroPasta> {
    validar_com(
        p,
        |p| fs2::available_space(p),
        |real| {
            let mut prova = tempfile::NamedTempFile::new_in(real)?;
            prova.write_all(b"teste de escrita")?;
            prova.as_file().sync_all()
        },
    )
}
fn validar_com(
    p: &Path,
    espaco: impl FnOnce(&Path) -> std::io::Result<u64>,
    escrever: impl FnOnce(&Path) -> std::io::Result<()>,
) -> std::result::Result<AvisoPasta, ErroPasta> {
    let absoluto = std::path::absolute(p)?;
    if absoluto.to_string_lossy().chars().count() > 120 || absoluto.parent().is_none() {
        return Err(ErroPasta::CaminhoInvalido);
    }
    fs::create_dir_all(&absoluto)?;
    let real = fs::canonicalize(&absoluto)?;
    if real.parent().is_none() {
        return Err(ErroPasta::CaminhoInvalido);
    }
    escrever(&real)?;
    conferir_espaco(espaco(&real)?, ESPACO_MINIMO)?;
    if !real.join(".reliquias").is_file() && fs::read_dir(&real)?.next().transpose()?.is_some() {
        return Ok(AvisoPasta::NaoVazia);
    }
    if [&absoluto, &real].iter().any(|p| {
        p.components().any(|c| {
            let s = c.as_os_str().to_string_lossy().to_lowercase();
            s == "dropbox" || s == "onedrive" || s.starts_with("onedrive - ")
        })
    }) {
        return Ok(AvisoPasta::Sincronizada);
    }
    Ok(AvisoPasta::Nenhum)
}
pub fn pasta_sugerida() -> PathBuf {
    #[cfg(windows)]
    if let Some(documentos) = documentos_windows() {
        return documentos.join("Reliquias");
    }
    PathBuf::from(
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .unwrap_or_default(),
    )
    .join("Documents/Reliquias")
}
#[cfg(windows)]
fn documentos_windows() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    #[repr(C)]
    struct Guid {
        a: u32,
        b: u16,
        c: u16,
        d: [u8; 8],
    }
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHGetKnownFolderPath(
            id: *const Guid,
            flags: u32,
            token: *mut std::ffi::c_void,
            path: *mut *mut u16,
        ) -> i32;
    }
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoTaskMemFree(p: *mut std::ffi::c_void);
    }
    let id = Guid {
        a: 0xfdd39ad0,
        b: 0x238f,
        c: 0x46af,
        d: [0xad, 0xb4, 0x6c, 0x85, 0x48, 0x03, 0x69, 0xc7],
    };
    let mut p = std::ptr::null_mut();
    // A API fornece uma string UTF-16 terminada em zero, alocada pelo COM.
    unsafe {
        let status = SHGetKnownFolderPath(&id, 0, std::ptr::null_mut(), &mut p);
        if status < 0 || p.is_null() {
            if !p.is_null() {
                CoTaskMemFree(p.cast());
            }
            return None;
        }
        let mut len = 0;
        while *p.add(len) != 0 {
            len += 1;
        }
        let resultado = std::ffi::OsString::from_wide(std::slice::from_raw_parts(p, len));
        CoTaskMemFree(p.cast());
        Some(PathBuf::from(resultado))
    }
}
impl Instancia {
    pub fn em(pasta: impl Into<PathBuf>) -> Self {
        Self { raiz: pasta.into() }
    }
}
fn hash(p: &Path) -> Result<Vec<u8>> {
    let mut f = File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_vec())
}
fn copiar_verificado(
    origem: &Path,
    destino: &Path,
    copiar: impl FnOnce(&Path, &Path) -> std::io::Result<u64>,
) -> Result<u64> {
    let antes = hash(origem)?;
    let bytes = copiar(origem, destino)?;
    OpenOptions::new().write(true).open(destino)?.sync_all()?;
    if hash(destino)? != antes || hash(origem)? != antes {
        return Err(Erro::HashInvalido);
    }
    Ok(bytes)
}
fn inventario(
    base: &Path,
    atual: &Path,
    arquivos: &mut Vec<PathBuf>,
    pastas: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(atual)? {
        let entry = entry?;
        let p = entry.path();
        let meta = fs::symlink_metadata(&p)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err(Erro::CaminhoInvalido);
            }
        }
        if meta.file_type().is_symlink() {
            return Err(Erro::CaminhoInvalido);
        }
        let rel = p
            .strip_prefix(base)
            .map_err(|_| Erro::CaminhoInvalido)?
            .to_path_buf();
        if rel == Path::new("launcher.lock") {
            continue;
        }
        if meta.is_dir() {
            pastas.push(rel);
            inventario(base, &p, arquivos, pastas)?;
        } else if meta.is_file() {
            arquivos.push(rel);
        } else {
            return Err(Erro::CaminhoInvalido);
        }
    }
    Ok(())
}
pub async fn mover_instancia(de: &Path, para: &Path, prog: Sender<Progresso>) -> Result<()> {
    mover_com_config(de, para, prog, None).await
}
/// Publica a configuração antes de remover os arquivos antigos.
pub async fn mover_com_config(
    de: &Path,
    para: &Path,
    prog: Sender<Progresso>,
    config: Option<config::Config>,
) -> Result<()> {
    let de = de.to_path_buf();
    let para = para.to_path_buf();
    tokio::task::spawn_blocking(move || mover(&de, &para, prog, config))
        .await
        .map_err(|_| Erro::Instalacao)?
}
fn mover(
    de: &Path,
    para: &Path,
    prog: Sender<Progresso>,
    config: Option<config::Config>,
) -> Result<()> {
    let de = fs::canonicalize(de)?;
    let para = resolver(&std::path::absolute(para)?)?;
    if de.starts_with(&para) || para.starts_with(&de) {
        return Err(Erro::CaminhoInvalido);
    }
    validar_pasta(&para)?;
    let _origem = Instancia::em(&de).trancar()?;
    let _destino = Instancia::em(&para).trancar()?;
    if fs::read_dir(&para)?.any(|e| e.map_or(true, |e| e.file_name() != "launcher.lock")) {
        return Err(Erro::CaminhoInvalido);
    }
    let mut arquivos = Vec::new();
    let mut pastas = Vec::new();
    inventario(&de, &de, &mut arquivos, &mut pastas)?;
    arquivos.sort();
    let tamanho = arquivos.iter().try_fold(0u64, |n, p| {
        fs::metadata(de.join(p)).map(|m| n.saturating_add(m.len()))
    })?;
    conferir_espaco(fs2::available_space(&para)?, tamanho.max(ESPACO_MINIMO))?;
    let stage = tempfile::tempdir_in(&para)?;
    for p in &pastas {
        fs::create_dir_all(stage.path().join(p))?;
    }
    let mut bytes = 0;
    for (i, p) in arquivos.iter().enumerate() {
        let origem = de.join(p);
        let destino = stage.path().join(p);
        bytes += copiar_verificado(&origem, &destino, |de, para| fs::copy(de, para))?;
        let _ = prog.try_send(Progresso {
            concluidos: i + 1,
            total: arquivos.len(),
            bytes,
        });
    }
    if !stage.path().join(".reliquias").exists() {
        fs::write(stage.path().join(".reliquias"), b"")?;
    }
    let mut publicados = Vec::new();
    let config_anterior = if config.is_some() {
        Some(config::Config::carregar()?)
    } else {
        None
    };
    let publicar = (|| -> Result<()> {
        for entry in fs::read_dir(stage.path())? {
            let entry = entry?;
            let destino = para.join(entry.file_name());
            fs::rename(entry.path(), &destino)?;
            publicados.push(destino);
        }
        if let Some(c) = config {
            c.salvar()?;
        }
        Ok(())
    })();
    if let Err(e) = publicar {
        for p in publicados {
            fs::rename(
                &p,
                stage
                    .path()
                    .join(p.file_name().ok_or(Erro::CaminhoInvalido)?),
            )?;
        }
        return Err(e);
    }
    let limpar = (|| -> Result<()> {
        for p in &arquivos {
            fs::remove_file(de.join(p))?;
        }
        for p in pastas.iter().rev() {
            fs::remove_dir(de.join(p))?;
        }
        Ok(())
    })();
    if let Err(e) = limpar {
        for p in &pastas {
            fs::create_dir_all(de.join(p))?;
        }
        for p in &arquivos {
            if !de.join(p).try_exists()? {
                copiar_verificado(&para.join(p), &de.join(p), |de, para| fs::copy(de, para))?;
            }
        }
        if let Some(anterior) = config_anterior {
            anterior.salvar()?;
        }
        // Só descarta a cópia depois de restaurar a origem e o ponteiro.
        // Se a restauração falhar, conserva a cópia publicada para recuperação.
        for p in publicados {
            fs::rename(
                &p,
                stage
                    .path()
                    .join(p.file_name().ok_or(Erro::CaminhoInvalido)?),
            )?;
        }
        return Err(e);
    }
    drop(_origem);
    fs::remove_file(de.join("launcher.lock"))?;
    fs::remove_dir(&de)?;
    Ok(())
}

fn resolver(p: &Path) -> Result<PathBuf> {
    match fs::canonicalize(p) {
        Ok(real) => Ok(real),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let pai = p.parent().ok_or(Erro::CaminhoInvalido)?;
            let nome = p.file_name().ok_or(Erro::CaminhoInvalido)?;
            Ok(resolver(pai)?.join(nome))
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod testes_pasta {
    use super::*;
    #[test]
    fn copia_corrompida_e_rejeitada() {
        let t = tempfile::tempdir().unwrap();
        let origem = t.path().join("origem");
        let destino = t.path().join("destino");
        fs::write(&origem, b"original").unwrap();
        let resultado = copiar_verificado(&origem, &destino, |_, p| {
            fs::write(p, b"corrompido")?;
            Ok(10)
        });
        assert!(matches!(resultado, Err(Erro::HashInvalido)));
        assert_eq!(fs::read(origem).unwrap(), b"original");
    }
    #[test]
    fn pouco_espaco_e_sem_permissao() {
        let t = tempfile::tempdir().unwrap();
        assert!(
            matches!(validar_com(t.path(), |_| Ok(ESPACO_MINIMO - 1), |_| Ok(())),
            Err(ErroPasta::SemEspaco { livre, preciso }) if livre == ESPACO_MINIMO - 1 && preciso == ESPACO_MINIMO)
        );
        assert!(matches!(
            validar_com(
                t.path(),
                |_| Ok(ESPACO_MINIMO),
                |_| Err(std::io::ErrorKind::PermissionDenied.into())
            ),
            Err(ErroPasta::SemPermissao)
        ));
        assert_eq!(
            validar_com(t.path(), |_| Ok(ESPACO_MINIMO), |_| Ok(())).unwrap(),
            AvisoPasta::Nenhum
        );
    }
}
