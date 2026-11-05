use launcher_core::{
    contrato::Manifesto,
    instancia::{config::Config, Instancia},
    interface, pack,
};
fn manifesto() -> Manifesto {
    serde_json::from_str(include_str!("../../../contrato/manifesto.json")).unwrap()
}
#[test]
fn defaults_e_filtro() {
    assert!(Config::default().visual);
    assert!(
        serde_json::from_str::<Config>(r#"{"shader":true}"#)
            .unwrap()
            .visual
    );
    let m = manifesto();
    assert_eq!(m.filtrado(true), m);
    let base = m.filtrado(false);
    assert!(base.arquivos.iter().all(|a| a.grupo != "visual"));
    assert!(base.arquivos.len() < m.arquivos.len());
    let mut antigo = serde_json::to_value(&m.arquivos[0]).unwrap();
    antigo.as_object_mut().unwrap().remove("grupo");
    assert_eq!(
        serde_json::from_value::<launcher_core::contrato::Arquivo>(antigo)
            .unwrap()
            .grupo,
        "base"
    );
}
#[test]
fn plano_remove_visuais() {
    let d = tempfile::tempdir().unwrap();
    let m = manifesto();
    for a in &m.arquivos {
        let p = d.path().join(&a.caminho);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }
    let p = pack::plano(&m.filtrado(false), d.path()).unwrap();
    for a in m.arquivos.iter().filter(|a| a.grupo == "visual") {
        assert!(p.remover.contains(&d.path().join(&a.caminho)));
    }
    assert!(p.baixar.iter().all(|a| a.grupo != "visual"));
}
#[test]
fn options_e_shader() {
    let d = tempfile::tempdir().unwrap();
    let inst = Instancia::em(d.path());
    let mut m = manifesto();
    let mut a = m.arquivos[0].clone();
    a.grupo = "visual".into();
    a.caminho = "resourcepacks/beleza.zip".into();
    m.arquivos = vec![a];
    interface::aplicar_visual(&inst, true, &m).unwrap();
    assert!(!d.path().join("options.txt").exists());
    let p = d.path().join("options.txt");
    std::fs::write(&p,"musica:0.5\r\nresourcePacks:[\"vanilla\",\"file/reliquias-ptbr\",\"file/outro.zip\"]\r\nfov:90").unwrap();
    interface::aplicar_visual(&inst, true, &m).unwrap();
    let esperado="musica:0.5\r\nresourcePacks:[\"vanilla\",\"file/beleza.zip\",\"file/reliquias-ptbr\",\"file/outro.zip\"]\r\nfov:90";
    assert_eq!(std::fs::read_to_string(&p).unwrap(), esperado);
    interface::aplicar_visual(&inst, true, &m).unwrap();
    assert_eq!(std::fs::read_to_string(&p).unwrap(), esperado);
    interface::aplicar_shader(&inst, true).unwrap();
    interface::aplicar_visual(&inst, false, &m).unwrap();
    assert!(!std::fs::read_to_string(&p).unwrap().contains("beleza.zip"));
    assert!(
        std::fs::read_to_string(d.path().join("config/iris.properties"))
            .unwrap()
            .contains("enableShaders=true")
    );
    std::fs::write(&p, "resourcePacks:[\"vanilla\"]\nultima:1").unwrap();
    interface::aplicar_visual(&inst, true, &m).unwrap();
    assert_eq!(
        std::fs::read_to_string(p).unwrap(),
        "resourcePacks:[\"vanilla\",\"file/beleza.zip\"]\nultima:1"
    );
}

#[test]
fn options_sem_lista_ganha_apenas_a_linha_de_pacotes() {
    let d = tempfile::tempdir().unwrap();
    let inst = Instancia::em(d.path());
    let mut m = manifesto();
    let mut a = m.arquivos[0].clone();
    a.grupo = "visual".into();
    a.caminho = "resourcepacks/beleza.zip".into();
    m.arquivos = vec![a];
    let p = d.path().join("options.txt");
    std::fs::write(&p, "fov:90\r\n").unwrap();
    interface::aplicar_visual(&inst, false, &m).unwrap();
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "fov:90\r\n");
    interface::aplicar_visual(&inst, true, &m).unwrap();
    assert_eq!(
        std::fs::read_to_string(&p).unwrap(),
        "fov:90\r\nresourcePacks:[\"file/beleza.zip\"]\r\n"
    );
}

#[test]
fn visual_so_mods_preserva_base_e_shader() {
    let d = tempfile::tempdir().unwrap();
    let inst = Instancia::em(d.path());
    let mut m = manifesto();
    let mut a = m.arquivos[0].clone();
    a.grupo = "visual".into();
    a.caminho = "mods/voxy.jar".into();
    let mut base = a.clone();
    base.grupo = "base".into();
    base.caminho = "resourcepacks/texturas.zip".into();
    m.arquivos = vec![a, base];
    let options = "resourcePacks:[ \"vanilla\", \"file/texturas.zip\" ]\r\nfov:90\r\n";
    let p = d.path().join("options.txt");
    std::fs::write(&p, options).unwrap();
    for visual in [false, true] {
        interface::aplicar_visual(&inst, visual, &m).unwrap();
        assert!(!d.path().join("config/iris.properties").exists());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), options);
    }
    for shader in [true, false] {
        interface::aplicar_shader(&inst, shader).unwrap();
        for visual in [false, true] {
            interface::aplicar_visual(&inst, visual, &m).unwrap();
            let iris = std::fs::read_to_string(d.path().join("config/iris.properties")).unwrap();
            assert!(iris.contains(&format!("enableShaders={shader}")));
            assert_eq!(std::fs::read_to_string(&p).unwrap(), options);
        }
    }
}
