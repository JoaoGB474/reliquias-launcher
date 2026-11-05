//! NBT sem compressão. Valores opacos preservam os bytes originais, inclusive
//! strings Java (UTF-8 modificado), NaNs e campos desconhecidos do Minecraft.
use std::io::{Error, ErrorKind, Result};

pub(crate) struct Tag {
    pub nome: Vec<u8>,
    pub tipo: u8,
    pub valor: Valor,
}
pub(crate) enum Valor {
    Bytes(Vec<u8>),
    Lista(u8, Vec<Valor>),
    Composto(Vec<Tag>),
}
fn invalido() -> Error {
    Error::new(ErrorKind::InvalidData, "NBT inválido ou excede os limites")
}
struct Leitor<'a> {
    dados: &'a [u8],
    pos: usize,
}
impl Leitor<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8]> {
        let fim = self.pos.checked_add(n).ok_or_else(invalido)?;
        let b = self.dados.get(self.pos..fim).ok_or_else(invalido)?;
        self.pos = fim;
        Ok(b)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }
    fn texto(&mut self) -> Result<Vec<u8>> {
        let b = self.bytes(2)?;
        let n = u16::from_be_bytes([b[0], b[1]]) as usize;
        Ok(self.bytes(n)?.to_vec())
    }
    fn tamanho(&mut self) -> Result<usize> {
        let b = self.bytes(4)?;
        let n = i32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        let n = usize::try_from(n).map_err(|_| invalido())?;
        if n > self.dados.len().saturating_sub(self.pos) {
            return Err(invalido());
        }
        Ok(n)
    }
    fn valor(&mut self, tipo: u8, nivel: usize) -> Result<Valor> {
        if nivel > 64 {
            return Err(invalido());
        }
        let inicio = self.pos;
        match tipo {
            1 => {
                self.bytes(1)?;
            }
            2 => {
                self.bytes(2)?;
            }
            3 | 5 => {
                self.bytes(4)?;
            }
            4 | 6 => {
                self.bytes(8)?;
            }
            7 | 11 | 12 => {
                let n = self.tamanho()?;
                let largura = match tipo {
                    11 => 4,
                    12 => 8,
                    _ => 1,
                };
                self.bytes(n.checked_mul(largura).ok_or_else(invalido)?)?;
            }
            8 => {
                self.texto()?;
            }
            9 => {
                let elemento = self.byte()?;
                let n = self.tamanho()?;
                if elemento > 12 || (elemento == 0 && n != 0) {
                    return Err(invalido());
                }
                let mut lista = Vec::new();
                for _ in 0..n {
                    lista.push(self.valor(elemento, nivel + 1)?);
                }
                return Ok(Valor::Lista(elemento, lista));
            }
            10 => {
                let mut tags = Vec::new();
                loop {
                    let tipo = self.byte()?;
                    if tipo == 0 {
                        break;
                    }
                    let nome = self.texto()?;
                    tags.push(Tag {
                        nome,
                        tipo,
                        valor: self.valor(tipo, nivel + 1)?,
                    });
                }
                return Ok(Valor::Composto(tags));
            }
            _ => return Err(invalido()),
        }
        Ok(Valor::Bytes(self.dados[inicio..self.pos].to_vec()))
    }
}
pub(crate) fn ler(dados: &[u8]) -> Result<Tag> {
    let mut l = Leitor { dados, pos: 0 };
    let tipo = l.byte()?;
    if tipo != 10 {
        return Err(invalido());
    }
    let nome = l.texto()?;
    let valor = l.valor(tipo, 0)?;
    if l.pos != dados.len() {
        return Err(invalido());
    }
    Ok(Tag { nome, tipo, valor })
}
fn texto_bytes(b: &[u8], saida: &mut Vec<u8>) -> Result<()> {
    saida.extend(
        u16::try_from(b.len())
            .map_err(|_| invalido())?
            .to_be_bytes(),
    );
    saida.extend(b);
    Ok(())
}
fn escrever_tag(tag: &Tag, saida: &mut Vec<u8>) -> Result<()> {
    saida.push(tag.tipo);
    texto_bytes(&tag.nome, saida)?;
    escrever_valor(&tag.valor, saida)
}
fn escrever_valor(valor: &Valor, saida: &mut Vec<u8>) -> Result<()> {
    match valor {
        Valor::Bytes(b) => saida.extend(b),
        Valor::Composto(tags) => {
            for tag in tags {
                escrever_tag(tag, saida)?;
            }
            saida.push(0);
        }
        Valor::Lista(tipo, lista) => {
            saida.push(*tipo);
            saida.extend(
                i32::try_from(lista.len())
                    .map_err(|_| invalido())?
                    .to_be_bytes(),
            );
            for v in lista {
                escrever_valor(v, saida)?;
            }
        }
    }
    Ok(())
}
pub(crate) fn escrever(tag: &Tag) -> Result<Vec<u8>> {
    let mut saida = Vec::new();
    escrever_tag(tag, &mut saida)?;
    Ok(saida)
}
/// DataOutput.writeUTF: codifica cada unidade UTF-16, incluindo NUL e surrogates.
pub(crate) fn string(texto: &str) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    for c in texto.encode_utf16() {
        match c {
            1..=127 => b.push(c as u8),
            0..=2047 => b.extend([0xc0 | (c >> 6) as u8, 0x80 | (c & 63) as u8]),
            _ => b.extend([
                0xe0 | (c >> 12) as u8,
                0x80 | ((c >> 6) & 63) as u8,
                0x80 | (c & 63) as u8,
            ]),
        }
    }
    let mut saida = Vec::new();
    texto_bytes(&b, &mut saida)?;
    Ok(saida)
}
