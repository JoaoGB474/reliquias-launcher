use launcher_core::args::{montar_args, Contexto, VersaoJson};
#[test]
fn argumentos_preservam_espacos_regras_e_token_falso() {
    let v:VersaoJson=serde_json::from_value(serde_json::json!({"id":"1.21.1-neoforge-21.1.265","mainClass":"cpw.mods.bootstraplauncher.BootstrapLauncher","arguments":{"jvm":["-cp","${classpath}",{"rules":[{"action":"allow","os":{"name":"windows"}}],"value":"-Dos=windows"},{"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"-Ddemo=true"}],"game":["--username","${auth_player_name}","--accessToken","${auth_access_token}","--gameDir","${game_directory}"]}})).unwrap();
    let ctx = Contexto {
        java: "java.exe".into(),
        raiz: r"C:\Users\João da Silva\OneDrive\x".into(),
        usuario: "Akiri".into(),
        uuid: "abc".into(),
        access_falso: "x".into(),
        agent: r"C:\Users\João da Silva\agent.jar".into(),
        proxy_url: "http://127.0.0.1:1234/ygg".into(),
        memoria_mb: 4096,
        classpath: vec!["a.jar".into()],
        prefetched: "e30=".into(),
    };
    let args = montar_args(&v, &ctx);
    assert!(args.contains(&"-Xmx4096M".into()));
    assert!(args.contains(&"-Dauthlibinjector.yggdrasil.prefetched=e30=".into()));
    assert!(args.contains(&format!(
        "-javaagent:{}={}",
        ctx.agent.display(),
        ctx.proxy_url
    )));
    assert!(args.contains(&ctx.raiz.to_string_lossy().into_owned()));
    assert!(!args.contains(&"-Ddemo=true".into()));
    assert_eq!(args.contains(&"-Dos=windows".into()), cfg!(windows));
}
#[test]
fn heranca_preserva_argumentos_e_substitui_bibliotecas() {
    let base:VersaoJson=serde_json::from_value(serde_json::json!({"id":"base","mainClass":"Base","arguments":{"game":["--base"]},"libraries":[{"name":"x:y:1"}]})).unwrap();
    let child:VersaoJson=serde_json::from_value(serde_json::json!({"id":"neo","inheritsFrom":"base","mainClass":"Neo","arguments":{"game":["--neo"]},"libraries":[{"name":"x:y:2"}]})).unwrap();
    let m = base.herdar(child);
    assert_eq!(m.main_class, "Neo");
    assert_eq!(m.arguments.game.len(), 2);
    assert_eq!(m.libraries.len(), 1);
    assert_eq!(m.libraries[0]["name"], "x:y:2");
}
#[test]
fn regras_respeitam_versao_do_so_e_features() {
    use launcher_core::args::permitido_em;
    let r = serde_json::json!([{"action":"allow","os":{"name":"windows","version":"^10\\.","arch":"amd64"},"features":{"is_demo_user":true}}]);
    let f = std::collections::HashMap::from([("is_demo_user".into(), true)]);
    assert!(permitido_em(Some(&r), "windows", "amd64", "10.0.19045", &f));
    assert!(!permitido_em(Some(&r), "windows", "amd64", "6.1", &f));
    assert!(!permitido_em(
        Some(&r),
        "windows",
        "amd64",
        "10.0",
        &Default::default()
    ));
}

#[test]
fn perfil_de_desempenho_usa_zgc_geracional_e_ignora_gc_do_json() {
    use launcher_core::args::jvm_desempenho;
    let p = jvm_desempenho(6144);
    for esperado in [
        "-Xms2048M",
        "-Xmx6144M",
        "-XX:+UseZGC",
        "-XX:+ZGenerational",
        "-XX:SoftMaxHeapSize=4915M",
        "-Dfile.encoding=UTF-8",
    ] {
        assert!(
            p.contains(&esperado.to_string()),
            "faltou {esperado}: {p:?}"
        );
    }
    assert!(!p.iter().any(|a| a.contains("G1")), "nada de G1 no perfil");
    // PC pequeno: -Xms nunca passa do -Xmx
    assert!(jvm_desempenho(1536).contains(&"-Xms1536M".to_string()));
    // o JSON da versão tenta impor o G1: o coletor do perfil vence, sem dois coletores
    let v: VersaoJson = serde_json::from_value(serde_json::json!({"id":"x","mainClass":"M","arguments":{"jvm":["-XX:+UseG1GC","-XX:G1NewSizePercent=20","-XX:HeapDumpPath=dump","-XX:+UseParallelGC"],"game":[]}})).unwrap();
    let ctx = Contexto {
        java: "java.exe".into(),
        raiz: "C:\\x".into(),
        usuario: "a".into(),
        uuid: "b".into(),
        access_falso: "x".into(),
        agent: "agent.jar".into(),
        proxy_url: "http://127.0.0.1:1/ygg".into(),
        memoria_mb: 6144,
        classpath: vec![],
        prefetched: "e30=".into(),
    };
    let args = montar_args(&v, &ctx);
    assert_eq!(
        args.iter()
            .filter(|a| a.starts_with("-XX:+Use") && a.ends_with("GC"))
            .count(),
        1,
        "{args:?}"
    );
    assert!(args.contains(&"-XX:+UseZGC".to_string()));
    assert!(
        args.contains(&"-XX:HeapDumpPath=dump".to_string()),
        "o resto do JSON continua valendo"
    );
}
