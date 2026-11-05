#[tokio::test]
#[ignore = "baixa o runtime oficial da Mojang pela internet"]
async fn java_oficial_21() {
    let dir = tempfile::tempdir().unwrap();
    let inst = launcher_core::instancia::Instancia {
        raiz: dir.path().to_path_buf(),
    };
    let (tx, _) = tokio::sync::mpsc::channel(100);
    let java = inst.garantir_java(21, tx).await.unwrap();
    let out = tokio::process::Command::new(java)
        .arg("-version")
        .output()
        .await
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("21."));
}
#[tokio::test]
#[ignore = "requer API da Parte 1 com manifesto real, SPKI e internet"]
async fn instalar_pack_completo() {
    let cliente = launcher_core::cliente::Cliente::desenvolvimento().unwrap();
    let m = cliente.get_json("/rl/v1/manifesto", None).await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let inst = launcher_core::instancia::Instancia {
        raiz: dir.path().to_path_buf(),
    };
    let (tx, _) = tokio::sync::mpsc::channel(100);
    let instalado = launcher_core::mojang::instalar(&m, &inst, tx)
        .await
        .unwrap();
    let out = tokio::process::Command::new(instalado.java)
        .arg("-version")
        .output()
        .await
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("21."));
}
