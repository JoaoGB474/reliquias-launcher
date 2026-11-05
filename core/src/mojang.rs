use crate::{
    args::{permitido, VersaoJson},
    baixar::{baixar_todos, Hash, Item, Progresso},
    contrato::Manifesto,
    erro::{Erro, Result},
    instancia::Instancia,
    java::{json, Download},
    pack::caminho,
};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Sender;
pub struct Instalado {
    pub java: PathBuf,
    pub versao: VersaoJson,
    pub classpath: Vec<PathBuf>,
    pub agent: PathBuf,
}
fn item(d: Download, destino: PathBuf) -> Result<Item> {
    Ok(Item {
        url: d.url,
        destino,
        sha: Hash::sha1(&d.sha1)?,
        tamanho: Some(d.size),
        fallback: None,
    })
}
async fn garantir(itens: Vec<Item>, prog: Sender<Progresso>) -> Result<()> {
    let mut pendentes = Vec::new();
    for i in itens {
        if !i.destino.exists() || !i.sha.arquivo(&i.destino)? {
            pendentes.push(i);
        }
    }
    baixar_todos(pendentes, prog, 8).await
}
pub async fn instalar(
    m: &Manifesto,
    inst: &Instancia,
    prog: Sender<Progresso>,
) -> Result<Instalado> {
    let java = inst.garantir_java(m.java, prog.clone()).await?;
    let index: serde_json::Value =
        json("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json").await?;
    let entry = index["versions"]
        .as_array()
        .and_then(|a| a.iter().find(|v| v["id"] == m.minecraft))
        .ok_or(Erro::ConfiguracaoInvalida)?;
    let version_path = caminho(&inst.raiz, &format!("versions/{0}/{0}.json", m.minecraft))?;
    let url = entry["url"].as_str().ok_or(Erro::ConfiguracaoInvalida)?;
    garantir(
        vec![Item {
            url: url.into(),
            destino: version_path.clone(),
            sha: Hash::sha1(entry["sha1"].as_str().ok_or(Erro::HashInvalido)?)?,
            tamanho: None,
            fallback: None,
        }],
        prog.clone(),
    )
    .await?;
    let base: VersaoJson = serde_json::from_slice(&std::fs::read(version_path)?)?;
    let client_path = caminho(&inst.raiz, &format!("versions/{0}/{0}.jar", m.minecraft))?;
    garantir(
        vec![item(
            serde_json::from_value(base.downloads["client"].clone())?,
            client_path.clone(),
        )?],
        prog.clone(),
    )
    .await?;
    let asset_id = base.asset_index["id"]
        .as_str()
        .ok_or(Erro::ConfiguracaoInvalida)?;
    let asset_path = caminho(&inst.raiz, &format!("assets/indexes/{asset_id}.json"))?;
    garantir(
        vec![item(
            serde_json::from_value(base.asset_index.clone())?,
            asset_path.clone(),
        )?],
        prog.clone(),
    )
    .await?;
    let assets: serde_json::Value = serde_json::from_slice(&std::fs::read(asset_path)?)?;
    let mut itens = Vec::new();
    for obj in assets["objects"]
        .as_object()
        .ok_or(Erro::ConfiguracaoInvalida)?
        .values()
    {
        let hash = obj["hash"].as_str().ok_or(Erro::HashInvalido)?;
        let sha = Hash::sha1(hash)?;
        let path = format!("{}/{}", &hash[..2], hash);
        itens.push(Item {
            url: format!("https://resources.download.minecraft.net/{path}"),
            destino: inst.raiz.join("assets/objects").join(path),
            sha,
            tamanho: obj["size"].as_u64(),
            fallback: None,
        });
    }
    garantir(itens, prog.clone()).await?;
    let neo = crate::neoforge::instalar(&m.neoforge, inst, &java, prog.clone()).await?;
    let versao = base.herdar(neo);
    let mut cp = Vec::new();
    let mut libs = Vec::new();
    let mut natives = Vec::new();
    for lib in &versao.libraries {
        if !permitido(lib.get("rules")) {
            continue;
        }
        if let Some(artifact) = lib.pointer("/downloads/artifact") {
            let path = artifact["path"].as_str().ok_or(Erro::CaminhoInvalido)?;
            let destino = caminho(&inst.raiz.join("libraries"), path)?;
            cp.push(destino.clone());
            libs.push(item(serde_json::from_value(artifact.clone())?, destino)?);
        } else if let Some(name) = lib["name"].as_str() {
            let path = crate::neoforge::maven_path(name)?;
            let destino = caminho(&inst.raiz.join("libraries"), &path)?;
            if !destino.is_file() {
                return Err(Erro::Instalacao);
            }
            cp.push(destino);
        }
        let os = if cfg!(windows) { "windows" } else { "linux" };
        if let Some(classifier) = lib["natives"][os].as_str() {
            let classifier = classifier.replace(
                "${arch}",
                if cfg!(target_pointer_width = "64") {
                    "64"
                } else {
                    "32"
                },
            );
            let a = &lib["downloads"]["classifiers"][&classifier];
            let path = a["path"].as_str().ok_or(Erro::CaminhoInvalido)?;
            let destino = caminho(&inst.raiz.join("libraries"), path)?;
            natives.push(destino.clone());
            libs.push(item(serde_json::from_value(a.clone())?, destino)?);
        }
    }
    garantir(libs, prog.clone()).await?;
    for native in natives {
        extrair(&native, &inst.raiz.join("natives"))?;
    }
    // O JSON do NeoForge manda ignorar `${version_name}.jar` no módulo do jogo. Com `inheritsFrom`, o
    // jar vanilla se chama `1.21.1.jar` e entraria duas vezes (módulo `_1._21._1` x `minecraft`).
    // Por isso o jar do cliente vai pro classpath com o nome do perfil final.
    let jogo_jar = caminho(&inst.raiz, &format!("versions/{0}/{0}.jar", versao.id))?;
    if jogo_jar != client_path {
        let tamanho = |p: &std::path::Path| std::fs::metadata(p).map(|m| m.len()).ok();
        if tamanho(&jogo_jar) != tamanho(&client_path) {
            std::fs::copy(&client_path, &jogo_jar)?;
        }
    }
    cp.push(jogo_jar);
    let agent = caminho(&inst.raiz.join("agent"), &m.agent.nome)?;
    garantir(
        vec![Item {
            url: m.agent.url.clone(),
            destino: agent.clone(),
            sha: Hash::sha256(&m.agent.sha256)?,
            tamanho: None,
            fallback: None,
        }],
        prog,
    )
    .await?;
    Ok(Instalado {
        java,
        versao,
        classpath: cp,
        agent,
    })
}
fn extrair(arquivo: &Path, raiz: &Path) -> Result<()> {
    let mut zip =
        zip::ZipArchive::new(std::fs::File::open(arquivo)?).map_err(|_| Erro::Instalacao)?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|_| Erro::Instalacao)?;
        if f.is_dir() || f.name().starts_with("META-INF/") {
            continue;
        }
        let path = caminho(raiz, f.name())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let part = crate::baixar::parcial(&path);
        let mut out = std::fs::File::create(&part)?;
        std::io::copy(&mut f, &mut out)?;
        out.sync_all()?;
        drop(out);
        std::fs::rename(part, path)?;
    }
    Ok(())
}
