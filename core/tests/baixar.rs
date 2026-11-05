use launcher_core::baixar::{baixar_todos, Hash, Item};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[tokio::test]
async fn retoma_parcial_e_publica_apenas_apos_hash() {
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("mod.jar");
    std::fs::write(dir.path().join("mod.jar.part"), b"abcd").unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/arquivo", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0; 2048];
        let n = socket.read(&mut buf).await.unwrap();
        assert!(String::from_utf8_lossy(&buf[..n])
            .to_lowercase()
            .contains("range: bytes=4-"));
        socket.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 4-9/10\r\nContent-Length: 6\r\n\r\nefghij").await.unwrap();
    });
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let item = Item {
        url,
        destino: destino.clone(),
        sha: Hash::Sha256(Sha256::digest(b"abcdefghij").into()),
        tamanho: Some(10),
        fallback: None,
    };
    baixar_todos(vec![item], tx, 8).await.unwrap();
    server.await.unwrap();
    assert_eq!(std::fs::read(destino).unwrap(), b"abcdefghij");
    assert!(!dir.path().join("mod.jar.part").exists());
    let mut anterior = 0;
    while let Some(p) = rx.recv().await {
        assert!(p.bytes >= anterior);
        anterior = p.bytes;
    }
}
#[tokio::test]
async fn hash_errado_tenta_duas_vezes_sem_publicar() {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/arquivo", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut buf = [0; 2048];
            assert!(s.read(&mut buf).await.unwrap() > 0);
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nbad")
                .await
                .unwrap();
        }
    });
    let (tx, _) = tokio::sync::mpsc::channel(10);
    let result = baixar_todos(
        vec![Item {
            url,
            destino: dir.path().join("x.jar"),
            sha: Hash::Sha256([0; 32]),
            tamanho: Some(3),
            fallback: None,
        }],
        tx,
        8,
    )
    .await;
    assert!(matches!(
        result,
        Err(launcher_core::erro::Erro::HashInvalido)
    ));
    server.await.unwrap();
    assert!(!dir.path().join("x.jar").exists());
    assert!(!dir.path().join("x.jar.part").exists());
}
#[tokio::test]
async fn conexao_cortada_retoma_em_quarenta_por_cento() {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/x", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for tentativa in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0; 2048];
            let n = socket.read(&mut buf).await.unwrap();
            if tentativa == 0 {
                socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nabcd",
                    )
                    .await
                    .unwrap();
            } else {
                assert!(String::from_utf8_lossy(&buf[..n]).contains("bytes=4-"));
                socket.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 4-9/10\r\nContent-Length: 6\r\n\r\nefghij").await.unwrap();
            }
        }
    });
    let (tx, _) = tokio::sync::mpsc::channel(10);
    baixar_todos(
        vec![Item {
            url,
            destino: dir.path().join("x"),
            sha: Hash::Sha256(Sha256::digest(b"abcdefghij").into()),
            tamanho: Some(10),
            fallback: None,
        }],
        tx,
        8,
    )
    .await
    .unwrap();
    server.await.unwrap();
}
#[tokio::test]
async fn oito_downloads_progresso_monotonico() {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/x", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut sockets = Vec::new();
        for _ in 0..8 {
            sockets.push(listener.accept().await.unwrap().0);
        }
        for mut s in sockets {
            let mut buf = [0; 2048];
            assert!(s.read(&mut buf).await.unwrap() > 0);
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx")
                .await
                .unwrap();
        }
    });
    let itens = (0..8)
        .map(|i| Item {
            url: url.clone(),
            destino: dir.path().join(i.to_string()),
            sha: Hash::Sha256(Sha256::digest(b"x").into()),
            tamanho: Some(1),
            fallback: None,
        })
        .collect();
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        baixar_todos(itens, tx, 8),
    )
    .await
    .unwrap()
    .unwrap();
    server.await.unwrap();
    let mut anterior = 0;
    let mut concluidos = 0;
    while let Some(p) = rx.recv().await {
        assert!(p.bytes >= anterior);
        anterior = p.bytes;
        concluidos = p.concluidos;
    }
    assert_eq!((anterior, concluidos), (8, 8));
}

mod suporte;
#[tokio::test]
async fn fallback_apenas_erro_http_nunca_hash_errado() {
    use launcher_core::baixar::Fallback;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let chamadas = Arc::new(AtomicUsize::new(0));
    let calls = chamadas.clone();
    let (cliente, job) = suporte::servidor(move |p, _| {
        assert_eq!(p, "/rl/v1/arquivo/hash");
        calls.fetch_add(1, Ordering::SeqCst);
        (200, b"ok".to_vec())
    })
    .await;
    for status in [503, 200] {
        let dir = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/x", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for _ in 0..if status == 503 { 1 } else { 2 } {
                let (mut s, _) = listener.accept().await.unwrap();
                let mut buf = [0; 2048];
                assert!(s.read(&mut buf).await.unwrap() > 0);
                s.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Length: 2\r\nConnection: close\r\n\r\nxx").as_bytes()).await.unwrap();
            }
        });
        let (tx, _) = tokio::sync::mpsc::channel(10);
        let result = baixar_todos(
            vec![Item {
                url,
                destino: dir.path().join("x"),
                sha: Hash::Sha256(Sha256::digest(b"ok").into()),
                tamanho: Some(2),
                fallback: Some(Fallback {
                    cliente: cliente.clone(),
                    caminho: "/rl/v1/arquivo/hash".into(),
                }),
            }],
            tx,
            8,
        )
        .await;
        if status == 503 {
            assert!(result.is_ok());
        } else {
            assert!(matches!(
                result,
                Err(launcher_core::erro::Erro::HashInvalido)
            ));
        }
        server.await.unwrap();
    }
    assert_eq!(chamadas.load(Ordering::SeqCst), 1);
    job.abort();
}
#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "requer tmpfs de 1 MiB em RL_TEST_DISK_FULL_DIR"]
async fn disco_cheio_nao_publica() {
    let dir = tempfile::tempdir_in(std::env::var_os("RL_TEST_DISK_FULL_DIR").unwrap()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/x", listener.local_addr().unwrap());
    let job = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut b = [0; 1024];
        assert!(s.read(&mut b).await.unwrap() > 0);
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2097152\r\n\r\n")
            .await
            .unwrap();
        let _ = s.write_all(&vec![0; 2097152]).await;
    });
    let (tx, _) = tokio::sync::mpsc::channel(8);
    let result = baixar_todos(
        vec![Item {
            url,
            destino: dir.path().join("x"),
            sha: Hash::Sha256([0; 32]),
            tamanho: Some(2097152),
            fallback: None,
        }],
        tx,
        1,
    )
    .await;
    assert!(matches!(result, Err(launcher_core::erro::Erro::SemEspaco)));
    assert!(!dir.path().join("x").exists());
    job.await.unwrap();
}
#[tokio::test]
async fn mesmo_arquivo_repetido_baixa_uma_vez_e_hash_diferente_no_mesmo_destino_falha() {
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("assets/aa/aabb");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/a", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        // aceita uma única conexão: um segundo download do mesmo arquivo travaria o teste
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0; 2048];
        let _ = socket.read(&mut buf).await.unwrap();
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc")
            .await
            .unwrap();
    });
    let item = |sha: &[u8]| Item {
        url: url.clone(),
        destino: destino.clone(),
        sha: Hash::Sha256(Sha256::digest(sha).into()),
        tamanho: None,
        fallback: None,
    };
    let (tx, _rx) = tokio::sync::mpsc::channel(100);
    let conflito = baixar_todos(vec![item(b"abc"), item(b"outro")], tx.clone(), 8).await;
    assert!(
        conflito.is_err(),
        "mesmo destino com hash diferente deve falhar"
    );
    baixar_todos(vec![item(b"abc"), item(b"abc"), item(b"abc")], tx, 8)
        .await
        .unwrap();
    server.await.unwrap();
    assert_eq!(std::fs::read(destino).unwrap(), b"abc");
}
