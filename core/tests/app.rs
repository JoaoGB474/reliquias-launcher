use launcher_core::{
    erro::Erro,
    interface::{limites_memoria, saida_jogo},
};
#[test]
fn falha_do_jogo_nao_e_instalacao() {
    assert!(saida_jogo(Some(0)).is_ok());
    assert!(matches!(
        saida_jogo(Some(7)),
        Err(Erro::JogoSaiu { codigo: Some(7) })
    ));
    assert_eq!(
        saida_jogo(Some(7)).unwrap_err().to_string(),
        "O jogo fechou com erro (código 7)"
    );
}
#[test]
fn memoria_reserva_dois_gigas_e_respeita_core() {
    assert_eq!(limites_memoria(16384), 14336);
    assert_eq!(limites_memoria(4096), 2048);
    assert_eq!(limites_memoria(1024), 2048);
    assert_eq!(limites_memoria(131072), 65536);
}

#[test]
fn pasta_alheia_usa_subpasta_sem_alterar_conteudo() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("foto.txt"), b"original").unwrap();
    let destino = launcher_core::interface::destino_seguro(t.path()).unwrap();
    assert_eq!(destino, t.path().join("Reliquias"));
    assert_eq!(
        std::fs::read(t.path().join("foto.txt")).unwrap(),
        b"original"
    );
    std::fs::write(destino.join("alheio.txt"), b"original").unwrap();
    assert!(launcher_core::interface::destino_seguro(t.path()).is_err());
}
#[test]
fn shader_preserva_demais_opcoes_e_nao_duplica_chave() {
    let t = tempfile::tempdir().unwrap();
    let inst = launcher_core::instancia::Instancia::em(t.path());
    std::fs::create_dir_all(t.path().join("config")).unwrap();
    let path = t.path().join("config/iris.properties");
    std::fs::write(&path, "shaderPack=MeuShader.zip\nenableShaders=true\n").unwrap();
    launcher_core::interface::aplicar_shader(&inst, false).unwrap();
    launcher_core::interface::aplicar_shader(&inst, true).unwrap();
    let texto = std::fs::read_to_string(path).unwrap();
    assert!(texto.contains("shaderPack=MeuShader.zip"));
    assert!(texto.contains("enableShaders=true"));
    assert_eq!(texto.matches("enableShaders=").count(), 1);
}
