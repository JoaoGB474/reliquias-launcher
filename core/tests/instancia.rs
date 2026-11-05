use launcher_core::{erro::Erro, instancia::Instancia};
#[test]
fn segunda_instancia_recusada_ate_liberar() {
    let dir = tempfile::tempdir().unwrap();
    let inst = Instancia {
        raiz: dir.path().to_path_buf(),
    };
    let trava = inst.trancar().unwrap();
    assert!(matches!(inst.trancar(), Err(Erro::JaAberto)));
    drop(trava);
    assert!(inst.trancar().is_ok());
}
#[tokio::test]
async fn runtime_verificado_reentrante_repara_corrupcao() {
    use launcher_core::java::{instalar_manifesto, Runtime};
    use sha1::{Digest, Sha1};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/java", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut buf = [0; 2048];
            assert!(s.read(&mut buf).await.unwrap() > 0);
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\njava")
                .await
                .unwrap();
        }
    });
    let manifest:Runtime=serde_json::from_value(serde_json::json!({"files":{"bin/java.exe":{"type":"file","executable":true,"downloads":{"raw":{"url":url,"sha1":format!("{:x}",Sha1::digest(b"java")),"size":4}}}}})).unwrap();
    for i in 0..3 {
        if i == 2 {
            std::fs::write(dir.path().join("bin/java.exe"), b"bad").unwrap();
        }
        let (tx, _) = tokio::sync::mpsc::channel(10);
        instalar_manifesto(&manifest, dir.path(), tx).await.unwrap();
    }
    server.await.unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("bin/java.exe")).unwrap(),
        b"java"
    );
}

#[test]
fn processo_travado_auxiliar() {
    if let Some(raiz) = std::env::var_os("RL_TEST_LOCK_ROOT") {
        let inst = Instancia { raiz: raiz.into() };
        let _trava = inst.trancar().unwrap();
        std::fs::write(inst.raiz.join("pronto"), b"ok").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
}
#[test]
fn trava_liberada_ao_matar_processo() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "processo_travado_auxiliar", "--nocapture"])
        .env("RL_TEST_LOCK_ROOT", dir.path())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let inicio = std::time::Instant::now();
    while !dir.path().join("pronto").exists()
        && inicio.elapsed() < std::time::Duration::from_secs(5)
    {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let inst = Instancia {
        raiz: dir.path().to_path_buf(),
    };
    let bloqueado = matches!(inst.trancar(), Err(Erro::JaAberto));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(bloqueado);
    assert!(inst.trancar().is_ok());
}
