use launcher_core::{cliente::Cliente, proxy::iniciar, sessao::Sessao};
use std::sync::Arc;
#[tokio::test]
async fn proxy_so_loopback_e_recusa_outros_caminhos() {
    let r: launcher_core::contrato::LoginResp =
        serde_json::from_str(include_str!("../../../contrato/login_ok.json")).unwrap();
    let s = Arc::new(tokio::sync::Mutex::new(Sessao::de_resposta(r).unwrap()));
    let proxy = iniciar(Cliente::desenvolvimento().unwrap(), s)
        .await
        .unwrap();
    assert_eq!(proxy.endereco().ip(), std::net::Ipv4Addr::LOCALHOST);
    let response = reqwest::get(format!("http://127.0.0.1:{}/rl/v1/login", proxy.porta))
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    proxy.parar().await;
}
mod suporte;
#[tokio::test]
async fn injeta_token_real_renova_uma_vez_e_preserva_metadados() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let joins = Arc::new(AtomicUsize::new(0));
    let count = joins.clone();
    let (c, job) = suporte::servidor(move |path, body| {
        if path == "/ygg/" {
            return (200, b"{\"meta\":true}".to_vec());
        }
        if path == "/rl/v1/refresh" {
            let mut r: serde_json::Value =
                serde_json::from_str(include_str!("../../../contrato/login_ok.json")).unwrap();
            r["access"] = "renovado".into();
            r["expira_em"] = u64::MAX.into();
            return (200, serde_json::to_vec(&r).unwrap());
        }
        assert_eq!(path, "/ygg/sessionserver/session/minecraft/join");
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let n = count.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            body["accessToken"],
            if n == 0 { "real" } else { "renovado" }
        );
        (if n == 0 { 401 } else { 204 }, vec![])
    })
    .await;
    let r = serde_json::from_str(include_str!("../../../contrato/login_ok.json")).unwrap();
    let mut s = Sessao::de_resposta(r).unwrap();
    s.access = "real".into();
    s.expira_em = u64::MAX;
    let cofre = Arc::new(suporte::CofreMemoria::default());
    let p =
        launcher_core::proxy::iniciar_com(c, Arc::new(tokio::sync::Mutex::new(s)), cofre.clone())
            .await
            .unwrap();
    let http = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}", p.porta);
    assert_eq!(
        http.get(format!("{url}/ygg/"))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
        b"{\"meta\":true}"[..]
    );
    assert_eq!(
        http.post(format!("{url}/ygg/sessionserver/session/minecraft/join"))
            .json(&serde_json::json!({"accessToken":"x","selectedProfile":"abc"}))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(joins.load(Ordering::SeqCst), 2);
    assert_eq!(cofre.salvos.load(Ordering::SeqCst), 1);
    p.parar().await;
    job.abort();
}
