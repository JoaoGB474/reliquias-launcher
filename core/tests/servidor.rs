use launcher_core::{instancia::Instancia, interface::garantir_servidor};
use std::fs;

// NBT montado à mão, independente do leitor/escritor de produção.
fn texto(nome: &str, valor: &str) -> Vec<u8> {
    let mut b = vec![8];
    b.extend((nome.len() as u16).to_be_bytes());
    b.extend(nome.as_bytes());
    b.extend((valor.len() as u16).to_be_bytes());
    b.extend(valor.as_bytes());
    b
}
fn entrada(nome: &str, ip: &str) -> Vec<u8> {
    let mut b = texto("name", nome);
    b.extend(texto("ip", ip));
    b.extend([1, 0, 6]);
    b.extend(b"hidden");
    b.push(1);
    b.extend(texto("icon", "dados do ícone"));
    b.push(0);
    b
}
fn arquivo(entradas: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![10, 0, 0, 9, 0, 7];
    b.extend(b"servers");
    b.push(10);
    b.extend((entradas.len() as i32).to_be_bytes());
    for e in entradas {
        b.extend(e);
    }
    b.extend(texto("extra", "preservado"));
    b.push(0);
    b
}
#[test]
fn ausente_e_idempotente() {
    let t = tempfile::tempdir().unwrap();
    let inst = Instancia::em(t.path().join("nova"));
    garantir_servidor(&inst, "Relíquias SMP", "localhost:25565").unwrap();
    let p = inst.raiz.join("servers.dat");
    let antes = fs::read(&p).unwrap();
    let mut esperado = vec![10, 0, 0, 9, 0, 7];
    esperado.extend(b"servers");
    esperado.extend([10, 0, 0, 0, 1]);
    esperado.extend(texto("name", "Relíquias SMP"));
    esperado.extend(texto("ip", "localhost:25565"));
    esperado.extend([0, 0]);
    assert_eq!(antes, esperado);
    garantir_servidor(&inst, "Relíquias SMP", "localhost:25565").unwrap();
    assert_eq!(fs::read(p).unwrap(), antes);
}
#[test]
fn insere_primeiro_preservando_outros() {
    let t = tempfile::tempdir().unwrap();
    let inst = Instancia::em(t.path());
    let outra = entrada("Outro", "outro:25565");
    fs::write(
        t.path().join("servers.dat"),
        arquivo(std::slice::from_ref(&outra)),
    )
    .unwrap();
    garantir_servidor(&inst, "Novo", "novo").unwrap();
    let mut nova = texto("name", "Novo");
    nova.extend(texto("ip", "novo"));
    nova.push(0);
    assert_eq!(
        fs::read(t.path().join("servers.dat")).unwrap(),
        arquivo(&[nova, outra])
    );
}
#[test]
fn mesmo_ip_duas_vezes_so_corrige_nomes() {
    let t = tempfile::tempdir().unwrap();
    let inst = Instancia::em(t.path());
    let p = t.path().join("servers.dat");
    let outra = entrada("Outro", "outro");
    fs::write(
        &p,
        arquivo(&[
            outra.clone(),
            entrada("Velho", "alvo"),
            entrada("Duplicado", "alvo"),
        ]),
    )
    .unwrap();
    for _ in 0..2 {
        garantir_servidor(&inst, "Atual", "alvo").unwrap();
    }
    assert_eq!(
        fs::read(p).unwrap(),
        arquivo(&[outra, entrada("Atual", "alvo"), entrada("Atual", "alvo")])
    );
}
#[test]
fn corrompido_e_guardado() {
    let t = tempfile::tempdir().unwrap();
    let inst = Instancia::em(t.path());
    let p = t.path().join("servers.dat");
    for b in [
        vec![1, 2, 3],
        vec![
            10, 0, 0, 9, 0, 7, b's', b'e', b'r', b'v', b'e', b'r', b's', 10, 127, 255, 255, 255,
        ],
    ] {
        fs::write(&p, &b).unwrap();
        garantir_servidor(&inst, "Novo", "novo").unwrap();
        assert_eq!(fs::read(t.path().join("servers.dat.bak")).unwrap(), b);
        let antes = fs::read(&p).unwrap();
        garantir_servidor(&inst, "Novo", "novo").unwrap();
        assert_eq!(fs::read(&p).unwrap(), antes);
    }
}
#[test]
fn manifesto_antigo_e_novo() {
    let mut valor: serde_json::Value =
        serde_json::from_str(include_str!("../../../contrato/manifesto.json")).unwrap();
    valor.as_object_mut().unwrap().remove("servidor");
    let antigo: launcher_core::contrato::Manifesto = serde_json::from_value(valor.clone()).unwrap();
    assert!(antigo.servidor.is_none());
    valor["servidor"] = serde_json::json!({"nome":"Relíquias SMP","endereco":"localhost"});
    let novo: launcher_core::contrato::Manifesto = serde_json::from_value(valor).unwrap();
    assert_eq!(novo.servidor.unwrap().endereco, "localhost");
}

#[test]
fn preserva_todos_os_tipos_nbt_e_texto_java() {
    let t = tempfile::tempdir().unwrap();
    let inst = Instancia::em(t.path());
    let mut outra = entrada("Outro", "outro");
    outra.pop();
    // Campos sem nome, um de cada tipo numérico, arrays, lista e composto.
    for (tipo, bytes) in [(1, 1), (2, 2), (3, 4), (4, 8), (5, 4), (6, 8)] {
        outra.extend([tipo, 0, 0]);
        outra.extend(vec![255; bytes]);
    }
    for (tipo, largura) in [(7, 1), (11, 4), (12, 8)] {
        outra.extend([tipo, 0, 0, 0, 0, 0, 2]);
        outra.extend(vec![127; largura * 2]);
    }
    outra.extend([9, 0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 42, 0, 0, 0, 43]);
    outra.extend([10, 0, 0, 8, 0, 0, 0, 2, 0xc0, 0x80, 0]);
    outra.push(0);
    let p = t.path().join("servers.dat");
    fs::write(&p, arquivo(std::slice::from_ref(&outra))).unwrap();
    garantir_servidor(&inst, "\0😀", "novo").unwrap();
    let mut nova = vec![
        8, 0, 4, b'n', b'a', b'm', b'e', 0, 8, 0xc0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80,
    ];
    nova.extend(texto("ip", "novo"));
    nova.push(0);
    assert_eq!(fs::read(&p).unwrap(), arquivo(&[nova, outra]));
    assert!(!t.path().join("servers.dat.bak").exists());
}
