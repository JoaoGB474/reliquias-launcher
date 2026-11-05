use crate::{
    args::VersaoJson,
    baixar::{baixar_todos, Hash, Item, Progresso},
    erro::{Erro, Result},
    instancia::Instancia,
    pack::caminho,
};
use std::path::Path;
use tokio::sync::mpsc::Sender;
pub fn maven_path(name: &str) -> Result<String> {
    let p: Vec<_> = name.split(':').collect();
    if p.len() < 3 || p.len() > 4 {
        return Err(Erro::ConfiguracaoInvalida);
    }
    Ok(format!(
        "{}/{}/{}/{}-{}{}.jar",
        p[0].replace('.', "/"),
        p[1],
        p[2],
        p[1],
        p[2],
        p.get(3).map(|c| format!("-{c}")).unwrap_or_default()
    ))
}
pub async fn instalar(
    versao: &str,
    inst: &Instancia,
    java: &Path,
    prog: Sender<Progresso>,
) -> Result<VersaoJson> {
    let id = format!("neoforge-{versao}");
    let output = caminho(&inst.raiz, &format!("versions/{id}/{id}.json"))?;
    if !output.exists() {
        let url=format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{versao}/neoforge-{versao}-installer.jar");
        let sha = crate::java::http()?
            .get(format!("{url}.sha1"))
            .send()
            .await
            .map_err(Erro::rede)?
            .error_for_status()
            .map_err(Erro::rede)?
            .text()
            .await
            .map_err(Erro::rede)?;
        let installer = caminho(&inst.raiz, &format!("installers/neoforge-{versao}.jar"))?;
        baixar_todos(
            vec![Item {
                url,
                destino: installer.clone(),
                sha: Hash::sha1(sha.trim())?,
                tamanho: None,
                fallback: None,
            }],
            prog,
            1,
        )
        .await?;
        let profiles = inst.raiz.join("launcher_profiles.json");
        if !profiles.exists() {
            let part = crate::baixar::parcial(&profiles);
            std::fs::write(&part, b"{\"profiles\":{}}")?;
            std::fs::rename(part, profiles)?;
        }
        let mut comando = tokio::process::Command::new(java);
        comando
            .arg("-jar")
            .arg(installer)
            .arg("--installClient")
            .arg(&inst.raiz)
            .current_dir(&inst.raiz);
        #[cfg(windows)]
        comando.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: sem terminal durante a instalação
        let status = comando.status().await?;
        if !status.success() {
            return Err(Erro::Instalacao);
        }
    }
    Ok(serde_json::from_slice(&std::fs::read(output)?)?)
}
