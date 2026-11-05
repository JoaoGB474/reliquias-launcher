use launcher_core::instancia::{
    config::Config, mover_instancia, validar_pasta, AvisoPasta, ErroPasta, Instancia,
};
use std::fs;
#[test]
fn cria_valida_e_identifica_pastas() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("nova");
    assert_eq!(validar_pasta(&p).unwrap(), AvisoPasta::Nenhum);
    assert!(p.is_dir());
    fs::write(p.join("pessoal"), b"intocado").unwrap();
    assert_eq!(validar_pasta(&p).unwrap(), AvisoPasta::NaoVazia);
    fs::write(p.join(".reliquias"), b"").unwrap();
    assert_eq!(validar_pasta(&p).unwrap(), AvisoPasta::Nenhum);
    assert_eq!(
        validar_pasta(&t.path().join("OneDrive/jogo")).unwrap(),
        AvisoPasta::Sincronizada
    );
    assert!(matches!(
        validar_pasta(&t.path().join("x".repeat(121))),
        Err(ErroPasta::CaminhoInvalido)
    ));
    #[cfg(windows)]
    assert!(validar_pasta(std::path::Path::new("C:\\")).is_err());
}
#[test]
fn config_corrompida_e_escrita_atomica() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("config.json");
    fs::write(&p, b"invalido").unwrap();
    assert_eq!(Config::carregar_de(&p).unwrap(), Config::default());
    let c = Config {
        pasta: Some(t.path().join("jogo")),
        memoria_mb: Some(4096),
        shader: true,
        visual: false,
    };
    c.salvar_em(&p).unwrap();
    assert_eq!(Config::carregar_de(&p).unwrap(), c);
    Config::default().salvar_em(&p).unwrap();
    assert_eq!(Config::carregar_de(&p).unwrap(), Config::default());
}
#[tokio::test]
async fn move_preserva_arquivos_e_recusa_sobreposicao() {
    let t = tempfile::tempdir().unwrap();
    let de = t.path().join("origem");
    let para = t.path().join("destino");
    fs::create_dir_all(de.join("saves/vazio")).unwrap();
    fs::write(de.join("saves/mundo"), b"mundo importante").unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    assert!(mover_instancia(&de, &de.join("filha"), tx.clone())
        .await
        .is_err());
    assert!(!de.join("filha").exists());
    mover_instancia(&de, &para, tx).await.unwrap();
    assert!(!de.join("saves/mundo").exists());
    assert_eq!(
        fs::read(para.join("saves/mundo")).unwrap(),
        b"mundo importante"
    );
    assert!(para.join("saves/vazio").is_dir());
    assert!(rx.recv().await.is_some());
    assert!(Instancia::em(&para).trancar().is_ok());
}
#[tokio::test]
async fn destino_ocupado_ou_origem_trancada_preserva_origem() {
    let t = tempfile::tempdir().unwrap();
    let de = t.path().join("origem");
    let para = t.path().join("destino");
    fs::create_dir(&de).unwrap();
    fs::create_dir(&para).unwrap();
    fs::write(de.join("mundo"), b"original").unwrap();
    fs::write(para.join("pessoal"), b"pessoal").unwrap();
    let (tx, _) = tokio::sync::mpsc::channel(16);
    assert!(mover_instancia(&de, &para, tx.clone()).await.is_err());
    fs::remove_file(para.join("pessoal")).unwrap();
    let _trava = Instancia::em(&de).trancar().unwrap();
    assert!(mover_instancia(&de, &para, tx).await.is_err());
    assert_eq!(fs::read(de.join("mundo")).unwrap(), b"original");
}
#[test]
fn cli_preserva_legado_e_escolhe_subpasta_sem_rede() {
    let t = tempfile::tempdir().unwrap();
    let legado = t.path().join("Reliquias");
    fs::create_dir(&legado).unwrap();
    fs::write(legado.join("mundo"), b"legado").unwrap();
    let destino = t.path().join("espaço á");
    fs::create_dir(&destino).unwrap();
    fs::write(destino.join("pessoal"), b"pessoal").unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rl"))
        .arg("pasta")
        .arg(&destino)
        .env("APPDATA", t.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let c = Config::carregar_de(&t.path().join("Reliquias-Launcher/config.json")).unwrap();
    assert_eq!(fs::read(c.pasta.unwrap().join("mundo")).unwrap(), b"legado");
    assert_eq!(fs::read(destino.join("pessoal")).unwrap(), b"pessoal");
}

#[cfg(windows)]
#[tokio::test]
async fn falha_no_meio_da_copia_preserva_origem_e_limpa_destino() {
    use std::os::windows::fs::OpenOptionsExt;
    let t = tempfile::tempdir().unwrap();
    let de = t.path().join("origem");
    let para = t.path().join("destino");
    fs::create_dir(&de).unwrap();
    fs::write(de.join("a"), b"primeiro").unwrap();
    fs::write(de.join("z"), b"bloqueado").unwrap();
    let _bloqueio = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(de.join("z"))
        .unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    assert!(mover_instancia(&de, &para, tx).await.is_err());
    assert!(
        rx.recv().await.is_some(),
        "ao menos um arquivo foi copiado antes da falha"
    );
    assert_eq!(fs::read(de.join("a")).unwrap(), b"primeiro");
    assert!(!para.join("a").exists());
    assert_eq!(fs::read_dir(&para).unwrap().count(), 1); // apenas a trava
}
#[cfg(windows)]
#[test]
fn falha_ao_publicar_config_desfaz_migracao() {
    use std::os::windows::fs::OpenOptionsExt;
    let t = tempfile::tempdir().unwrap();
    let legado = t.path().join("Reliquias");
    fs::create_dir(&legado).unwrap();
    fs::write(legado.join("mundo"), b"intacto").unwrap();
    let cp = t.path().join("Reliquias-Launcher/config.json");
    Config::default().salvar_em(&cp).unwrap();
    // Permite leitura, mas impede a substituição atômica no Windows.
    let _bloqueio = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&cp)
        .unwrap();
    let destino = t.path().join("novo");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rl"))
        .arg("pasta")
        .arg(&destino)
        .env("APPDATA", t.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(fs::read(legado.join("mundo")).unwrap(), b"intacto");
    assert!(!destino.join("mundo").exists());
    assert_eq!(Config::carregar_de(&cp).unwrap(), Config::default());
}
#[test]
fn cli_config_corrompida_preserva_legado_e_reutiliza_escolha() {
    let t = tempfile::tempdir().unwrap();
    let legado = t.path().join("Reliquias");
    fs::create_dir(&legado).unwrap();
    fs::write(legado.join("mundo"), b"legado").unwrap();
    let cp = t.path().join("Reliquias-Launcher/config.json");
    fs::create_dir(cp.parent().unwrap()).unwrap();
    fs::write(&cp, b"{corrompido").unwrap();
    for nome in ["primeira", "segunda", "segunda"] {
        let destino = t.path().join(nome);
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_rl"))
            .arg("pasta")
            .arg(&destino)
            .env("APPDATA", t.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(destino.join("mundo")).unwrap(), b"legado");
    }
}

#[cfg(windows)]
#[tokio::test]
async fn falha_na_limpeza_restaura_arquivos_ja_removidos() {
    use std::os::windows::fs::OpenOptionsExt;
    let t = tempfile::tempdir().unwrap();
    let de = t.path().join("origem");
    let para = t.path().join("destino");
    fs::create_dir(&de).unwrap();
    fs::write(de.join("a"), b"primeiro").unwrap();
    fs::write(de.join("z"), b"ultimo").unwrap();
    let _bloqueio = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(de.join("z"))
        .unwrap();
    let (tx, _) = tokio::sync::mpsc::channel(16);
    assert!(mover_instancia(&de, &para, tx).await.is_err());
    assert_eq!(fs::read(de.join("a")).unwrap(), b"primeiro");
    assert_eq!(fs::read(de.join("z")).unwrap(), b"ultimo");
    assert!(!para.join("a").exists());
}
#[test]
fn memoria_padrao_6gb_e_jogador_pode_alterar() {
    use launcher_core::instancia::config::{Config, MEMORIA_PADRAO_MB};
    let mut c = Config::default();
    assert_eq!(MEMORIA_PADRAO_MB, 6144);
    assert_eq!(c.memoria_efetiva(), 6144, "sem escolha, o padrão é 6 GB");
    c.definir_memoria(Some(8192)).unwrap();
    assert_eq!(c.memoria_efetiva(), 8192);
    assert!(
        c.definir_memoria(Some(512)).is_err(),
        "abaixo de 2 GB é recusado"
    );
    assert!(c.definir_memoria(Some(10_000_000)).is_err());
    assert_eq!(
        c.memoria_efetiva(),
        8192,
        "valor inválido não altera a escolha"
    );
    c.definir_memoria(None).unwrap();
    assert_eq!(c.memoria_efetiva(), 6144);
    c.memoria_mb = Some(100); // config editada à mão
    assert_eq!(c.memoria_efetiva(), 2048);
}

#[test]
fn memoria_do_jogo_se_adapta_ao_pc_sem_passar_do_que_o_jogador_escolheu() {
    use launcher_core::instancia::config::Config;
    let c = Config::default();
    assert_eq!(c.memoria_para(16384), 6144, "PC grande: o padrão de 6 GB");
    assert_eq!(
        c.memoria_para(8192),
        6144,
        "8 GB: ainda cabe 6 GB com 2 GB livres"
    );
    assert_eq!(
        c.memoria_para(6144),
        4096,
        "6 GB de RAM: deixa 2 GB pro Windows"
    );
    assert_eq!(
        c.memoria_para(4096),
        2048,
        "PC pequeno: nunca abaixo de 2 GB"
    );
    assert_eq!(c.memoria_para(0), 6144, "RAM desconhecida: o padrão");
    let mut j = Config::default();
    j.definir_memoria(Some(10240)).unwrap();
    assert_eq!(j.memoria_para(32768), 10240, "a escolha do jogador vale");
    assert_eq!(
        j.memoria_para(8192),
        6144,
        "mas nunca passa de RAM menos 2 GB"
    );
}
