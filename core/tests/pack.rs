use launcher_core::{contrato::Manifesto, erro::Erro, pack::plano};
fn manifesto() -> Manifesto {
    serde_json::from_str(include_str!("../../../contrato/manifesto.json")).unwrap()
}
#[test]
fn recusa_caminhos_perigosos() {
    let dir = tempfile::tempdir().unwrap();
    for caminho in [
        "../x",
        "/abs",
        r"C:\x",
        "mods/CON",
        "mods/con.jar",
        "mods/a:stream",
        "mods/x.",
        "mods/../x",
    ] {
        let mut m = manifesto();
        m.arquivos[0].caminho = caminho.into();
        assert!(
            matches!(plano(&m, dir.path()), Err(Erro::CaminhoInvalido)),
            "{caminho}"
        );
    }
}
#[test]
fn remove_apenas_gerenciados_preserva_uma_vez() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("mods")).unwrap();
    std::fs::write(dir.path().join("mods/velho.jar"), b"x").unwrap();
    std::fs::write(dir.path().join("save.dat"), b"x").unwrap();
    std::fs::write(dir.path().join("options.txt"), b"custom").unwrap();
    let p = plano(&manifesto(), dir.path()).unwrap();
    assert_eq!(p.baixar.len(), manifesto().arquivos.len() - 1);
    assert_eq!(p.remover, vec![dir.path().join("mods/velho.jar")]);
}
#[tokio::test]
async fn mesmo_manifesto_nao_baixa_nada() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let mut m = manifesto();
    m.arquivos.truncate(1);
    m.arquivos[0].sha256 = format!("{:x}", Sha256::digest(b"ok"));
    m.arquivos[0].tamanho = 2;
    std::fs::create_dir(dir.path().join("mods")).unwrap();
    std::fs::write(dir.path().join(&m.arquivos[0].caminho), b"ok").unwrap();
    let cliente = launcher_core::cliente::Cliente::desenvolvimento().unwrap();
    for _ in 0..2 {
        let p = plano(&m, dir.path()).unwrap();
        assert!(p.baixar.is_empty() && p.remover.is_empty());
        let (tx, _) = tokio::sync::mpsc::channel(8);
        assert_eq!(
            launcher_core::pack::sincronizar(&m, dir.path(), &cliente, tx)
                .await
                .unwrap()
                .bytes,
            0
        );
    }
}
mod suporte;
#[tokio::test]
async fn falha_de_download_nao_remove_mod_antigo() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("mods")).unwrap();
    std::fs::write(dir.path().join("mods/velho.jar"), b"antigo").unwrap();
    let mut m = manifesto();
    m.arquivos.truncate(1);
    m.arquivos[0].origem = launcher_core::contrato::Origem::Servidor;
    let (c, job) = suporte::servidor(|_, _| (503, vec![])).await;
    let (tx, _) = tokio::sync::mpsc::channel(8);
    assert!(launcher_core::pack::sincronizar(&m, dir.path(), &c, tx)
        .await
        .is_err());
    assert_eq!(
        std::fs::read(dir.path().join("mods/velho.jar")).unwrap(),
        b"antigo"
    );
    job.abort();
}
