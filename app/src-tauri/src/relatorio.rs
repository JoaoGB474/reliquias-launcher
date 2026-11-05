use launcher_core::{
    erro::{Erro, Result},
    interface, sessao,
};
use regex::Regex;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

// Remove valores conhecidos mesmo fora de campos e descarta linhas de credenciais,
// inclusive tokens antigos que já não estão no cofre.
fn sanitizar(texto: &str, segredos: &[&str]) -> Result<String> {
    let mut texto = texto.to_owned();
    for segredo in segredos.iter().filter(|s| !s.is_empty()) {
        texto = texto.replace(segredo, "[removido]");
    }
    let bearer = Regex::new(r"(?im)^.*(?:authorization|bearer\s).*$")
        .map_err(|_| Erro::ConfiguracaoInvalida)?;
    texto = bearer
        .replace_all(&texto, "[credencial removida]")
        .into_owned();
    let campos = Regex::new(r#"(?is)(?:refresh(?:_?token)?|access(?:_?token)?|client[_-]?secret|authorization)[\"']?\s*[:=]\s*(?:\"[^\"]*\"|'[^']*'|[^\s,;}]+)"#)
        .map_err(|_| Erro::ConfiguracaoInvalida)?;
    texto = campos
        .replace_all(&texto, "[credencial removida]")
        .into_owned();
    let suspeito = Regex::new(r"(?i)refresh|access.?token|client.?secret|authorization|bearer\s|--access|\baccess\s*[:=]|eyJ[a-zA-Z0-9_-]+\.")
        .map_err(|_| Erro::ConfiguracaoInvalida)?;
    Ok(texto
        .lines()
        .map(|linha| {
            if suspeito.is_match(linha) {
                "[credencial removida]"
            } else {
                linha
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn ler_log(path: &Path) -> Result<String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok("Log ainda não disponível".into())
        }
        Err(e) => return Err(e.into()),
    };
    // Limite evita carregar logs arbitrariamente grandes na memória.
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn empacotar(raiz: &Path, desktop: &Path, hardware: &str, segredos: &[&str]) -> Result<PathBuf> {
    let mut temporario = tempfile::NamedTempFile::new_in(desktop)?;
    {
        let mut zip = zip::ZipWriter::new(temporario.as_file_mut());
        for (nome, texto) in [
            ("latest.log", ler_log(&raiz.join("logs/latest.log"))?),
            ("launcher.log", ler_log(&raiz.join("logs/launcher.log"))?),
            ("hardware.txt", hardware.to_owned()),
        ] {
            zip.start_file(
                nome,
                zip::write::FileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .map_err(std::io::Error::other)?;
            zip.write_all(sanitizar(&texto, segredos)?.as_bytes())?;
        }
        zip.finish().map_err(std::io::Error::other)?;
    }
    temporario.as_file().sync_all()?;
    let sufixo = temporario
        .path()
        .file_name()
        .ok_or(Erro::CaminhoInvalido)?
        .to_string_lossy();
    let destino = desktop.join(format!("Reliquias-relatorio-{sufixo}.zip"));
    temporario
        .persist_noclobber(&destino)
        .map_err(|e| Erro::Io(e.error))?;
    Ok(destino)
}

pub fn gerar(raiz: &Path, desktop: &Path) -> Result<PathBuf> {
    let hardware = format!("Sistema: {}\nArquitetura: {}\nProcessadores lógicos: {}\nMemória RAM: {} MB\nLauncher: {}\n",
        std::env::consts::OS, std::env::consts::ARCH,
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        interface::memoria_total_mb().unwrap_or(0), env!("CARGO_PKG_VERSION"));
    let sessao = sessao::carregar();
    let segredos = sessao
        .as_ref()
        .map(|s| vec![s.access.as_str(), s.refresh.as_str()])
        .unwrap_or_default();
    empacotar(raiz, desktop, &hardware, &segredos)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zip_sem_tokens_em_todos_os_arquivos() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("logs")).unwrap();
        let entrada = "linha útil\n{\"refresh\":\"REFRESH_FICTICIO_123\",\"access\":\"ACCESS_FICTICIO_456\",\"client_secret\":\"SECRET_FICTICIO_789\"}\nAuthorization: Bearer BEARER_FICTICIO_123\n--accessToken ARG_FICTICIO_123\nsolto ACCESS_FICTICIO_456\nclient_secret =\n\"MULTILINHA_FICTICIO_123\"\n";
        for nome in ["latest.log", "launcher.log"] {
            fs::write(dir.path().join("logs").join(nome), entrada).unwrap();
        }
        let path = empacotar(
            dir.path(),
            dir.path(),
            "RAM: 8192\nrefresh=HARDWARE_FICTICIO_123",
            &["ACCESS_FICTICIO_456", "REFRESH_FICTICIO_123"],
        )
        .unwrap();
        let re =
            Regex::new(r"(?:REFRESH|ACCESS|SECRET|BEARER|ARG|MULTILINHA|HARDWARE)_FICTICIO_\d+")
                .unwrap();
        assert!(!re.is_match(&String::from_utf8_lossy(&fs::read(&path).unwrap())));
        let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
        assert_eq!(zip.len(), 3);
        for nome in ["latest.log", "launcher.log", "hardware.txt"] {
            let mut texto = String::new();
            zip.by_name(nome)
                .unwrap()
                .read_to_string(&mut texto)
                .unwrap();
            assert!(!re.is_match(&texto), "vazamento em {nome}");
        }
    }
    #[test]
    fn logs_ausentes_ainda_geram_relatorio() {
        let dir = tempfile::tempdir().unwrap();
        assert!(empacotar(dir.path(), dir.path(), "RAM: 8192", &[]).is_ok());
    }
}
