use http_body_util::{BodyExt, Full};
use rcgen::{CertificateParams, KeyPair, PublicKeyData};
use sha2::{Digest, Sha256};
use std::sync::Arc;
pub async fn servidor(
    handler: impl Fn(String, Vec<u8>) -> (u16, Vec<u8>) + Send + Sync + 'static,
) -> (launcher_core::cliente::Cliente, tokio::task::JoinHandle<()>) {
    let key = KeyPair::generate().unwrap();
    let pin = Sha256::digest(key.subject_public_key_info()).into();
    let cert = CertificateParams::new(vec!["localhost".into()])
        .unwrap()
        .self_signed(&key)
        .unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![cert.der().clone()],
        rustls::pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
    )
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cliente =
        launcher_core::cliente::Cliente::novo(&listener.local_addr().unwrap().to_string(), pin)
            .unwrap();
    let handler = Arc::new(handler);
    let job = tokio::spawn(async move {
        let mut jobs = tokio::task::JoinSet::new();
        loop {
            tokio::select! {connection=listener.accept()=>{let(socket,_)=connection.unwrap();let acceptor=acceptor.clone();let handler=handler.clone();jobs.spawn(async move{let socket=acceptor.accept(socket).await.unwrap();let service=hyper::service::service_fn(move |req:hyper::Request<hyper::body::Incoming>|{let handler=handler.clone();async move{let path=req.uri().to_string();let body=req.into_body().collect().await.unwrap().to_bytes();let(status,body)=handler(path,body.to_vec());Ok::<_,std::convert::Infallible>(hyper::Response::builder().status(status).body(Full::new(hyper::body::Bytes::from(body))).unwrap())}});let _=hyper::server::conn::http1::Builder::new().serve_connection(hyper_util::rt::TokioIo::new(socket),service).await;});},_=jobs.join_next(),if !jobs.is_empty()=>{}}
        }
    });
    (cliente, job)
}
#[allow(dead_code)]
#[derive(Default)]
pub struct CofreMemoria {
    pub apagado: std::sync::atomic::AtomicBool,
    pub salvos: std::sync::atomic::AtomicUsize,
}
impl launcher_core::sessao::Cofre for CofreMemoria {
    fn guardar(&self, _: &launcher_core::sessao::Sessao) -> launcher_core::erro::Result<()> {
        self.salvos
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    fn apagar(&self) -> launcher_core::erro::Result<()> {
        self.apagado
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}
