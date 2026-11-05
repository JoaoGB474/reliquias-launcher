use launcher_core::{cliente::Cliente, erro::Erro};
use rcgen::{CertificateParams, KeyPair, PublicKeyData};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
async fn servidor(expirado: bool) -> (String, [u8; 32]) {
    let chave = KeyPair::generate().unwrap();
    let pin = Sha256::digest(chave.subject_public_key_info()).into();
    let mut params = CertificateParams::new(vec!["localhost".into()]).unwrap();
    if expirado {
        params.not_after = rcgen::date_time_ymd(2001, 1, 1);
        params.not_before = rcgen::date_time_ymd(2000, 1, 1);
    }
    let cert = params.self_signed(&chave).unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![cert.der().clone()],
        rustls::pki_types::PrivatePkcs8KeyDer::from(chave.serialize_der()).into(),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let host = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        if let Ok(mut tls) = tokio_rustls::TlsAcceptor::from(Arc::new(config))
            .accept(socket)
            .await
        {
            let mut buf = [0; 4096];
            let _ = tls.read(&mut buf).await;
            let _ = tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}").await;
        }
    });
    (host, pin)
}
#[tokio::test]
async fn aceita_chave_correta_inclusive_certificado_expirado() {
    for expirado in [false, true] {
        let (host, pin) = servidor(expirado).await;
        let cliente = Cliente::novo(&host, pin).unwrap();
        let resposta: serde_json::Value = cliente.get_json("/", None).await.unwrap();
        assert_eq!(resposta["ok"], true);
    }
}
#[tokio::test]
async fn recusa_outra_chave() {
    let (host, _) = servidor(false).await;
    let cliente = Cliente::novo(&host, [42; 32]).unwrap();
    assert!(matches!(
        cliente.get_json::<serde_json::Value>("/", None).await,
        Err(Erro::ServidorDesconhecido)
    ));
}
