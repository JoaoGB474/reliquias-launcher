//! Políticas compartilhadas pelos adaptadores de interface.
use crate::{
    baixar::Progresso,
    erro::{Erro, Result},
    instancia::{
        self,
        config::{Config, MEMORIA_MAX_MB, MEMORIA_MIN_MB},
        AvisoPasta, Instancia,
    },
};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Sender;

pub fn limites_memoria(total_mb: u32) -> u32 {
    total_mb
        .saturating_sub(2048)
        .clamp(MEMORIA_MIN_MB, MEMORIA_MAX_MB)
}
pub fn saida_jogo(codigo: Option<i32>) -> Result<()> {
    if codigo == Some(0) {
        Ok(())
    } else {
        Err(Erro::JogoSaiu { codigo })
    }
}

pub fn memoria_total_mb() -> Result<u32> {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Memoria {
            tamanho: u32,
            carga: u32,
            fisica: u64,
            livre: u64,
            page: u64,
            page_livre: u64,
            virtual_total: u64,
            virtual_livre: u64,
            extendida: u64,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GlobalMemoryStatusEx(p: *mut Memoria) -> i32;
        }
        let mut m = Memoria {
            tamanho: std::mem::size_of::<Memoria>() as u32,
            carga: 0,
            fisica: 0,
            livre: 0,
            page: 0,
            page_livre: 0,
            virtual_total: 0,
            virtual_livre: 0,
            extendida: 0,
        };
        // Estrutura C inicializada, com o tamanho exigido pela API do Windows.
        if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok((m.fisica / (1024 * 1024)).min(u32::MAX as u64) as u32)
    }
    #[cfg(not(windows))]
    {
        Err(Erro::ConfiguracaoInvalida)
    }
}
/// Nunca entrega uma pasta alheia ao sincronizador que remove arquivos gerenciados.
pub fn destino_seguro(p: &Path) -> Result<PathBuf> {
    let mut destino = std::path::absolute(p)?;
    if instancia::validar_pasta(&destino)? == AvisoPasta::NaoVazia {
        destino = destino.join("Reliquias");
        if instancia::validar_pasta(&destino)? == AvisoPasta::NaoVazia {
            return Err(Erro::CaminhoInvalido);
        }
    }
    Ok(destino)
}
pub async fn definir_pasta(p: &Path, mover: bool, prog: Sender<Progresso>) -> Result<()> {
    let cfg_path = Config::caminho()?;
    let _config_trava = Instancia::em(cfg_path.parent().ok_or(Erro::CaminhoInvalido)?).trancar()?;
    let mut cfg = Config::carregar()?;
    let origem = Instancia::padrao()?;
    let destino = destino_seguro(p)?;
    if origem.raiz.exists()
        && std::fs::canonicalize(&origem.raiz)? == std::fs::canonicalize(&destino)?
    {
        cfg.pasta = Some(destino);
        return cfg.salvar();
    }
    cfg.pasta = Some(destino.clone());
    if mover && origem.raiz.exists() {
        instancia::mover_com_config(&origem.raiz, &destino, prog, Some(cfg)).await
    } else {
        let _origem = if origem.raiz.exists() {
            Some(origem.trancar()?)
        } else {
            None
        };
        let _destino = Instancia::em(&destino).trancar()?;
        std::fs::write(destino.join(".reliquias"), b"")?;
        cfg.salvar()
    }
}
pub fn salvar_ajustes(
    memoria_mb: Option<u32>,
    shader: bool,
    visual: bool,
    total: u32,
) -> Result<()> {
    if memoria_mb.is_some_and(|mb| mb > limites_memoria(total)) {
        return Err(Erro::ConfiguracaoInvalida);
    }
    let p = Config::caminho()?;
    let _trava = Instancia::em(p.parent().ok_or(Erro::CaminhoInvalido)?).trancar()?;
    let mut cfg = Config::carregar()?;
    cfg.definir_memoria(memoria_mb)?;
    cfg.shader = shader;
    cfg.visual = visual;
    cfg.salvar()
}
pub fn aplicar_shader(inst: &Instancia, ligado: bool) -> Result<()> {
    use std::io::Write;
    let dir = inst.raiz.join("config");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("iris.properties");
    let texto = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let mut linhas: Vec<_> = texto
        .lines()
        .filter(|l| !l.trim_start().starts_with("enableShaders="))
        .map(str::to_owned)
        .collect();
    linhas.push(format!("enableShaders={ligado}"));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    writeln!(tmp, "{}", linhas.join("\n"))?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| Erro::Io(e.error))?;
    Ok(())
}

/// Ajusta apenas a lista de pacotes; preserva as demais linhas byte a byte.
pub fn aplicar_visual(
    inst: &Instancia,
    ligado: bool,
    manifesto: &crate::contrato::Manifesto,
) -> Result<()> {
    use std::io::Write;
    let path = inst.raiz.join("options.txt");
    let texto = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let mut visuais = Vec::new();
    for a in manifesto.arquivos.iter().filter(|a| a.grupo == "visual") {
        if let Some(nome) = a.caminho.strip_prefix("resourcepacks/") {
            let nome = nome.split('/').next().unwrap_or_default();
            let entrada = format!("file/{nome}");
            if !nome.is_empty() && !visuais.contains(&entrada) {
                visuais.push(entrada);
            }
        }
    }
    if visuais.is_empty() {
        return Ok(());
    }
    let mut novo = String::new();
    for linha in texto.split_inclusive('\n') {
        if let Some(lista) = linha.strip_prefix("resourcePacks:") {
            let fim = if linha.ends_with("\r\n") {
                "\r\n"
            } else if linha.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            let mut packs: Vec<String> = serde_json::from_str(lista.trim_end())?;
            packs.retain(|p| !visuais.contains(p));
            if ligado {
                let pos = packs
                    .iter()
                    .position(|p| p == "file/reliquias-ptbr" || p == "file/reliquias-ptbr.zip")
                    .unwrap_or(packs.len());
                packs.splice(pos..pos, visuais.clone());
            }
            novo.push_str("resourcePacks:");
            novo.push_str(&serde_json::to_string(&packs)?);
            novo.push_str(fim);
        } else {
            novo.push_str(linha);
        }
    }
    if ligado && !visuais.is_empty() && !texto.lines().any(|l| l.starts_with("resourcePacks:")) {
        let fim = if texto.contains("\r\n") { "\r\n" } else { "\n" };
        if !novo.is_empty() && !novo.ends_with('\n') {
            novo.push_str(fim);
        }
        novo.push_str("resourcePacks:");
        novo.push_str(&serde_json::to_string(&visuais)?);
        novo.push_str(fim);
    }
    if novo != texto {
        let mut tmp = tempfile::NamedTempFile::new_in(&inst.raiz)?;
        tmp.write_all(novo.as_bytes())?;
        tmp.as_file().sync_all()?;
        tmp.persist(path).map_err(|e| Erro::Io(e.error))?;
    }
    Ok(())
}

/// Mantém o servidor do pack na lista de Multijogador, preservando campos alheios.
/// O chamador deve manter a trava da instância durante a atualização.
pub fn garantir_servidor(inst: &Instancia, nome: &str, endereco: &str) -> Result<()> {
    use crate::nbt::{self, Tag, Valor};
    use std::{
        fs,
        io::{ErrorKind, Write},
    };
    let nome = nbt::string(nome)?;
    let ip = nbt::string(endereco)?;
    let caminho = inst.raiz.join("servers.dat");
    let anterior = match fs::read(&caminho) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let vazio = || Tag {
        nome: Vec::new(),
        tipo: 10,
        valor: Valor::Composto(vec![Tag {
            nome: b"servers".to_vec(),
            tipo: 9,
            valor: Valor::Lista(10, Vec::new()),
        }]),
    };
    let mut corrompido = false;
    let mut raiz = match anterior.as_deref().map(nbt::ler).transpose() {
        Ok(Some(tag)) if servidores_validos(&tag) => tag,
        Ok(None) => vazio(),
        _ => {
            corrompido = true;
            vazio()
        }
    };
    if let Valor::Composto(tags) = &mut raiz.valor {
        if let Some(Tag {
            valor: Valor::Lista(tipo, lista),
            ..
        }) = tags.iter_mut().find(|t| t.nome == b"servers")
        {
            *tipo = 10;
            let mut encontrou = false;
            for entrada in lista.iter_mut() {
                if let Valor::Composto(campos) = entrada {
                    let mesmo_ip = campos.iter().any(|t| {
                        t.nome == b"ip"
                            && t.tipo == 8
                            && matches!(&t.valor, Valor::Bytes(b) if b == &ip)
                    });
                    if mesmo_ip {
                        encontrou = true;
                        let mut tem_nome = false;
                        for campo in campos.iter_mut().filter(|t| t.nome == b"name") {
                            campo.tipo = 8;
                            campo.valor = Valor::Bytes(nome.clone());
                            tem_nome = true;
                        }
                        if !tem_nome {
                            campos.push(Tag {
                                nome: b"name".to_vec(),
                                tipo: 8,
                                valor: Valor::Bytes(nome.clone()),
                            });
                        }
                    }
                }
            }
            if !encontrou {
                lista.insert(
                    0,
                    Valor::Composto(vec![
                        Tag {
                            nome: b"name".to_vec(),
                            tipo: 8,
                            valor: Valor::Bytes(nome),
                        },
                        Tag {
                            nome: b"ip".to_vec(),
                            tipo: 8,
                            valor: Valor::Bytes(ip),
                        },
                    ]),
                );
            }
        }
    }
    let dados = nbt::escrever(&raiz)?;
    if anterior.as_deref() == Some(dados.as_slice()) {
        return Ok(());
    }
    fs::create_dir_all(&inst.raiz)?;
    let publicar = |destino: &Path, bytes: &[u8]| -> Result<()> {
        let mut temp = tempfile::NamedTempFile::new_in(&inst.raiz)?;
        temp.write_all(bytes)?;
        temp.as_file().sync_all()?;
        temp.persist(destino).map_err(|e| e.error)?;
        Ok(())
    };
    if corrompido {
        if let Some(bytes) = &anterior {
            publicar(&inst.raiz.join("servers.dat.bak"), bytes)?;
        }
    }
    publicar(&caminho, &dados)
}

fn servidores_validos(raiz: &crate::nbt::Tag) -> bool {
    use crate::nbt::Valor;
    let Valor::Composto(tags) = &raiz.valor else {
        return false;
    };
    let mut servidores = tags.iter().filter(|t| t.nome == b"servers");
    let valido = servidores.next().is_some_and(|t| {
        matches!(&t.valor,
        Valor::Lista(tipo, lista) if *tipo == 10 || (*tipo == 0 && lista.is_empty()))
    });
    valido && servidores.next().is_none()
}
