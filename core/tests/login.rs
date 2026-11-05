use launcher_core::login::{challenge, ouvir, validar_callback};
#[test]
fn pkce_rfc7636_e_state() {
    assert_eq!(
        challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    assert_eq!(
        validar_callback("/callback?code=abc&state=certo", "certo").unwrap(),
        "abc"
    );
    assert!(validar_callback("/callback?code=abc&state=errado", "certo").is_err());
    assert!(validar_callback("/callback?code=a&state=certo&state=errado", "certo").is_err());
}
#[tokio::test]
async fn escolhe_primeira_porta_livre_e_recusa_todas_ocupadas() {
    let a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let b = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let pa = a.local_addr().unwrap().port();
    let pb = b.local_addr().unwrap().port();
    assert!(matches!(
        ouvir(&[pa, pb]).await,
        Err(launcher_core::erro::Erro::PortasOcupadas)
    ));
    drop(b);
    let listener = ouvir(&[pa, pb]).await.unwrap();
    assert_eq!(listener.local_addr().unwrap().port(), pb);
}
#[test]
fn sem_perfil_exige_aprovacao() {
    let resp =
        serde_json::from_str(include_str!("../../../contrato/login_sem_perfil.json")).unwrap();
    assert!(matches!(
        launcher_core::sessao::Sessao::de_resposta(resp),
        Err(launcher_core::erro::Erro::SemAprovacao { .. })
    ));
}
mod suporte;
#[tokio::test]
async fn refresh_revogado_apaga_sessao() {
    let (c, job) = suporte::servidor(|path, _| {
        assert_eq!(path, "/rl/v1/refresh");
        (401, b"{\"erro\":\"revogada\"}".to_vec())
    })
    .await;
    let r = serde_json::from_str(include_str!("../../../contrato/login_ok.json")).unwrap();
    let mut s = launcher_core::sessao::Sessao::de_resposta(r).unwrap();
    s.expira_em = 0;
    let cofre = suporte::CofreMemoria::default();
    assert!(matches!(
        s.garantir_com(&c, &cofre).await,
        Err(launcher_core::erro::Erro::ContaRevogada)
    ));
    assert!(cofre.apagado.load(std::sync::atomic::Ordering::SeqCst));
    assert!(s.access.is_empty() && s.refresh.is_empty());
    job.abort();
}
