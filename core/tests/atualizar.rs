use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use launcher_core::atualizar::{verificar, Ultimo};
use sha2::{Digest, Sha256};
#[test]
fn assinatura_e_hash_precisam_conferir() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let bytes = b"executavel de teste";
    let meta = Ultimo {
        versao: "1.2.0".into(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        tamanho: bytes.len() as u64,
        assinatura: STANDARD.encode(key.sign(bytes).to_bytes()),
    };
    assert!(verificar(&meta, bytes, &key.verifying_key().to_bytes()).is_ok());
    assert!(verificar(&meta, b"alterado", &key.verifying_key().to_bytes()).is_err());
    assert!(verificar(
        &meta,
        bytes,
        &SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes()
    )
    .is_err());
    let mut m = meta;
    m.sha256 = "00".repeat(32);
    assert!(matches!(
        verificar(&m, bytes, &key.verifying_key().to_bytes()),
        Err(launcher_core::erro::Erro::AssinaturaInvalida)
    ));
}
mod suporte;
#[tokio::test]
async fn versao_igual_nao_baixa() {
    let(c,job)=suporte::servidor(|p,_|{assert_eq!(p,"/rl/v1/launcher/ultimo");(200,serde_json::to_vec(&serde_json::json!({"versao":"1.0.0","sha256":"00".repeat(32),"tamanho":0,"assinatura":""})).unwrap())}).await;
    assert!(
        launcher_core::atualizar::autoatualizar_com(&c, &"1.0.0".parse().unwrap(), [0; 32])
            .await
            .unwrap()
            .is_none()
    );
    job.abort();
}
#[tokio::test(start_paused = true)]
async fn vigia_so_notifica_sem_tocar_arquivos() {
    let (c, job) = suporte::servidor(|p, _| {
        assert_eq!(p, "/rl/v1/manifesto");
        (
            200,
            include_bytes!("../../../contrato/manifesto.json").to_vec(),
        )
    })
    .await;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let watcher = tokio::spawn(launcher_core::atualizar::vigiar_pack(c, 11, tx));
    tokio::task::yield_now().await;
    tokio::time::advance(std::time::Duration::from_secs(600)).await;
    // O TLS local precisa de tempo real; evita avançar automaticamente o timeout de rede.
    tokio::time::resume();
    let evento = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        evento,
        launcher_core::atualizar::Evento::AtualizacaoPronta { versao: 12 }
    );
    watcher.abort();
    job.abort();
}

#[tokio::test]
async fn vigia_resiste_a_erro_e_encerra_ao_desconectar() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let chamadas = Arc::new(AtomicUsize::new(0));
    let contador = chamadas.clone();
    let (sinal, mut chamadas_rx) = tokio::sync::mpsc::unbounded_channel();
    let (c, servidor) = suporte::servidor(move |_, _| {
        let n = contador.fetch_add(1, Ordering::SeqCst);
        let _ = sinal.send(());
        if n == 0 {
            (503, b"{}".to_vec())
        } else {
            (
                200,
                include_bytes!("../../../contrato/manifesto.json").to_vec(),
            )
        }
    })
    .await;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let vigia = tokio::spawn(launcher_core::atualizar::vigiar_pack(c, 11, tx));
    tokio::task::yield_now().await;
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(600)).await;
    tokio::time::resume();
    tokio::time::timeout(std::time::Duration::from_secs(5), chamadas_rx.recv())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(!vigia.is_finished());
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(600)).await;
    tokio::time::resume();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .unwrap(),
        Some(launcher_core::atualizar::Evento::AtualizacaoPronta { versao: 12 })
    );
    drop(rx);
    tokio::time::timeout(std::time::Duration::from_secs(1), vigia)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    servidor.abort();
}
