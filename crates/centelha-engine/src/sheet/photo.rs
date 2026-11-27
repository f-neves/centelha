//! O enquadramento da foto que o jogador envia para uma peça da própria ficha:
//! o zoom até 4 e o deslocamento. Port de `enqDe`, da barra de zoom, do botão
//! "reenquadrar" e da conta de `enquadraFoto` (`src/lib/ficha-engine.ts`).
//!
//! É a exceção da R31 ao corte de imagens (R10): só a foto do jogador na
//! ficha pode ser enquadrada com zoom, e o zoom a esconde em parte de
//! propósito, por escolha dele. Nenhuma outra imagem do projeto ganha zoom,
//! corte nem `object-fit: cover`.
//!
//! A tela (a barra, o arrasto, as medidas do quadro) fica com quem chama; aqui
//! entra a conta. Quem confere é o teste de paridade da ficha interativa.

use serde_json::{Map, Value, json};

use super::value::{is_str, string_to_number, to_number};
use crate::js;

/// O zoom máximo do enquadramento (`ZOOM_MAX`).
pub const ZOOM_MAX: f64 = 4.0;

/// O enquadramento de uma peça (`enqDe`): o zoom de 1 a [`ZOOM_MAX`] e o
/// deslocamento de -1 a 1 em cada eixo, como fração do que sobra do quadro.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhotoFrame {
    pub zoom: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

/// `Number(v) || d`, preso entre `lo` e `hi`: o `NaN` e o zero valem `d`.
fn clamped(v: Option<&Value>, d: f64, lo: f64, hi: f64) -> f64 {
    let n = to_number(v);
    let n = if n.is_nan() || n == 0.0 { d } else { n };
    js::max(lo, js::min(hi, n))
}

/// O zoom escrito num enquadramento (`Math.max(1, Math.min(ZOOM_MAX,
/// Number(enq?.z) || 1))`), como o desenho da foto e a barra o leem.
pub fn photo_zoom(enq: Option<&Value>) -> f64 {
    clamped(enq.and_then(|e| e.get("z")), 1.0, 1.0, ZOOM_MAX)
}

/// `enqDe`: o enquadramento da peça (`p.enq`), com tudo preso nos limites;
/// o que falta ou não é número vale o padrão (zoom 1, sem deslocamento).
pub fn photo_frame(piece: Option<&Value>) -> PhotoFrame {
    let enq = piece.and_then(|p| p.get("enq"));
    let at = |k: &str| enq.and_then(|e| e.get(k));
    PhotoFrame {
        zoom: photo_zoom(enq),
        offset_x: clamped(at("px"), 0.0, -1.0, 1.0),
        offset_y: clamped(at("py"), 0.0, -1.0, 1.0),
    }
}

/// A peça de uma chave do ajuste (`arm:<uid>` é de armadura, qualquer outra do
/// arsenal), para mudar.
fn piece_mut<'a>(sheet: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    let mut parts = key.split(':');
    let (kind, uid) = (parts.next()?, parts.next().unwrap_or(""));
    let list = if kind == "arm" {
        sheet.get_mut("equip")?.get_mut("armaduras")?
    } else {
        sheet.get_mut("arsenal")?
    };
    list.as_array_mut()?
        .iter_mut()
        .find(|p| is_str(p.get("uid"), uid))
}

fn object(v: &mut Value) -> &mut Map<String, Value> {
    if !v.is_object() {
        *v = Value::Object(Map::new());
    }
    match v {
        Value::Object(m) => m,
        _ => unreachable!(),
    }
}

/// O ouvinte da barra de zoom (`input[data-enq-z]`): `texto` é o valor da barra
/// (100 a 400, de 5 em 5). Guarda `{z, px, py}` na peça; no zoom 1 o
/// enquadramento volta ao padrão e não é guardado. Peça que não existe não muda
/// nada.
///
/// O texto que não é número (a barra nunca o dá) deixaria o zoom em `NaN`, que
/// o JSON grava como `null`.
pub fn set_photo_zoom(sheet: &mut Value, key: &str, text: &str) {
    let Some(piece) = piece_mut(sheet, key) else {
        return;
    };
    let zoom = js::max(1.0, js::min(ZOOM_MAX, string_to_number(text) / 100.0));
    let frame = photo_frame(Some(piece));
    let p = object(piece);
    if zoom == 1.0 {
        // enquadrada de novo: não guarda o que é padrão
        p.shift_remove("enq");
        return;
    }
    let z = serde_json::Number::from_f64(zoom).map_or(Value::Null, Value::Number);
    let enq = json!({"z": z, "px": frame.offset_x, "py": frame.offset_y});
    // A peça de armadura que o `normalize` da fonte monta guarda o lugar do
    // `enq` (logo antes de `vestida`), mesmo vazio; o JSON não guarda a chave
    // que não existe, então o `enq` entra onde a fonte o deixa. A peça do
    // arsenal não tem o lugar e o `enq` vai para o fim.
    if key.starts_with("arm:") && !p.contains_key("enq") {
        let at = p.keys().position(|k| k == "vestida").unwrap_or(p.len());
        p.shift_insert(at, "enq".into(), enq);
    } else {
        p.insert("enq".into(), enq);
    }
}

/// O botão "reenquadrar" (`[data-enq-reset]`): a peça volta ao enquadramento
/// padrão.
pub fn reset_photo_frame(sheet: &mut Value, key: &str) {
    if let Some(piece) = piece_mut(sheet, key) {
        object(piece).shift_remove("enq");
    }
}

/// Onde a foto fica no quadro (`enquadraFoto`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhotoPlacement {
    /// Quanto a foto ainda pode andar para cada lado, em pixels, já dividido
    /// pelo zoom (o deslocamento acontece dentro da escala).
    pub max_x: f64,
    pub max_y: f64,
    /// O deslocamento a aplicar, em pixels (`--tx` e `--ty`).
    pub shift_x: f64,
    pub shift_y: f64,
    /// Sobra o bastante para arrastar (a classe `desloca`).
    pub can_shift: bool,
}

/// `enquadraFoto`, a parte que é conta: com o tamanho natural da imagem e o do
/// quadro, `None` quando falta algum (a foto ainda não carregou). A imagem
/// cabe inteira no quadro (`object-fit: contain`) e daí parte a escala; a folga
/// é o módulo da diferença entre o desenho e o quadro, e não só o que passa dele,
/// para o arrasto valer também no zoom 100%.
pub fn photo_placement(
    frame: &PhotoFrame,
    natural: (f64, f64),
    quad: (f64, f64),
) -> Option<PhotoPlacement> {
    let ((nw, nh), (cw, ch)) = (natural, quad);
    let missing = |x: f64| x == 0.0 || x.is_nan();
    if missing(nw) || missing(nh) || missing(cw) {
        return None;
    }
    let k = js::min(cw / nw, ch / nh);
    let z = frame.zoom;
    let max_x = (nw * k * z - cw).abs() / 2.0 / z;
    let max_y = (nh * k * z - ch).abs() / 2.0 / z;
    Some(PhotoPlacement {
        max_x,
        max_y,
        shift_x: frame.offset_x * max_x,
        shift_y: frame.offset_y * max_y,
        can_shift: max_x > 0.5 || max_y > 0.5,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_defaults_to_no_zoom_and_no_offset() {
        let f = photo_frame(None);
        assert_eq!((f.zoom, f.offset_x, f.offset_y), (1.0, 0.0, 0.0));
        let f = photo_frame(Some(&json!({"enq": "texto"})));
        assert_eq!(f.zoom, 1.0);
    }

    #[test]
    fn the_zoom_stops_at_four_and_never_goes_below_one() {
        for (z, want) in [
            (json!(5), 4.0),
            (json!(4), 4.0),
            (json!(0.2), 1.0),
            (json!(-3), 1.0),
        ] {
            let f = photo_frame(Some(&json!({"enq": {"z": z}})));
            assert_eq!(f.zoom, want);
        }
        assert_eq!(photo_zoom(Some(&json!({"z": "2,5"}))), 1.0);
        assert_eq!(photo_zoom(Some(&json!({"z": "2.5"}))), 2.5);
    }

    #[test]
    fn a_zoom_that_is_not_a_number_is_stored_as_null() {
        let mut sheet = json!({"arsenal": [{"uid": "p1"}]});
        set_photo_zoom(&mut sheet, "ars:p1", "abc");
        assert_eq!(sheet["arsenal"][0]["enq"]["z"], Value::Null);
        // e lê de volta como o zoom padrão
        assert_eq!(photo_frame(Some(&sheet["arsenal"][0])).zoom, 1.0);
    }
}
