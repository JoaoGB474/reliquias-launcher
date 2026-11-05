use launcher_core::contrato::{ErroApi, LoginResp, Manifesto, Status};
use serde::{de::DeserializeOwned, Serialize};
fn conferir<T: DeserializeOwned + Serialize>(json: &str) {
    let original: serde_json::Value = serde_json::from_str(json).unwrap();
    let valor: T = serde_json::from_str(json).unwrap();
    assert_eq!(original, serde_json::to_value(valor).unwrap());
}
#[test]
fn contratos_preservam_todos_os_campos() {
    conferir::<ErroApi>(include_str!("../../../contrato/erro_sem_cargo.json"));
    conferir::<LoginResp>(include_str!("../../../contrato/login_ok.json"));
    conferir::<LoginResp>(include_str!("../../../contrato/login_sem_perfil.json"));
    conferir::<Manifesto>(include_str!("../../../contrato/manifesto.json"));
    conferir::<Status>(include_str!("../../../contrato/status.json"));
}

#[test]
fn novidades_preservam_contrato() {
    conferir::<launcher_core::contrato::Novidades>(include_str!(
        "../../../contrato/novidades.json"
    ));
}
mod suporte;
#[tokio::test]
async fn busca_novidades_por_tls() {
    let (cliente, servidor) = suporte::servidor(|caminho, corpo| {
        assert_eq!(caminho, "/rl/v1/novidades");
        assert!(corpo.is_empty());
        (
            200,
            include_bytes!("../../../contrato/novidades.json").to_vec(),
        )
    })
    .await;
    let novidades = cliente.novidades().await.unwrap();
    assert_eq!(novidades.servidor[0].versao, 5);
    assert_eq!(novidades.modpack[0].titulo, "EMI no lugar do JEI");
    servidor.abort();
}
