//! O ramo `arcano` de `regras.json`: as regras das Artes.
//!
//! O que entra em conta sai tipado (o custo por nível do improviso e da
//! Cura, o desconto da fonte do elemento, as linhas do esticar, os números da
//! manifestação e do desvio). O resto, que a página das regras só mostra, sai
//! como árvore de texto ([`Node`]) na ordem do arquivo, com leitura por
//! caminho (`get("improviso.graus")`, `text("cura.nota")`).
//!
//! O texto sai exatamente como está no JSON: sem `trim`, sem troca de
//! caractere. A árvore guarda as chaves de `arcano` fora o `tetoNivelArte`,
//! que tem campo próprio em [`crate::rules::ArcaneRules`].

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// Um valor da árvore de texto.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Um texto, como está no arquivo.
    Text(String),
    /// Um número.
    Number(f64),
    /// Um booleano (só onde o dado o tem, como `moldes.lista[3].curvavel`).
    Bool(bool),
    /// Um `null` do dado (como `hexagonos` de um molde sem hexágonos).
    Null,
    /// Uma lista, na ordem do arquivo.
    List(Vec<Node>),
    /// Um mapa, com as chaves na ordem do arquivo.
    Map(Vec<(String, Node)>),
}

impl Node {
    /// O texto, se for texto.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Node::Text(s) => Some(s),
            _ => None,
        }
    }

    /// O número, se for número.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Node::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// O booleano, se for booleano.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Node::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Os itens, se for lista.
    pub fn as_list(&self) -> Option<&[Node]> {
        match self {
            Node::List(v) => Some(v),
            _ => None,
        }
    }

    /// As entradas na ordem do arquivo, se for mapa.
    pub fn as_map(&self) -> Option<&[(String, Node)]> {
        match self {
            Node::Map(v) => Some(v),
            _ => None,
        }
    }

    /// É o `null` do dado?
    pub fn is_null(&self) -> bool {
        matches!(self, Node::Null)
    }

    /// A entrada de um mapa pela chave.
    pub fn entry(&self, key: &str) -> Option<&Node> {
        self.as_map()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// O item de uma lista pelo índice.
    pub fn item(&self, index: usize) -> Option<&Node> {
        self.as_list()?.get(index)
    }

    /// Lê por caminho: os segmentos separados por ponto, chave num mapa ou
    /// índice numa lista (`"improviso.graus"`, `"moldes.lista.3.curvavel"`).
    /// Caminho vazio dá o próprio nó. Caminho que não existe dá `None`.
    pub fn get(&self, path: &str) -> Option<&Node> {
        if path.is_empty() {
            return Some(self);
        }
        let mut node = self;
        for seg in path.split('.') {
            node = match node {
                Node::Map(_) => node.entry(seg)?,
                Node::List(_) => node.item(seg.parse().ok()?)?,
                _ => return None,
            };
        }
        Some(node)
    }

    /// O texto no caminho.
    pub fn text(&self, path: &str) -> Option<&str> {
        self.get(path)?.as_text()
    }

    /// O número no caminho.
    pub fn number(&self, path: &str) -> Option<f64> {
        self.get(path)?.as_number()
    }

    /// A lista no caminho.
    pub fn list(&self, path: &str) -> Option<&[Node]> {
        self.get(path)?.as_list()
    }

    /// O mapa no caminho.
    pub fn map(&self, path: &str) -> Option<&[(String, Node)]> {
        self.get(path)?.as_map()
    }
}

struct NodeVisitor;

impl<'de> Visitor<'de> for NodeVisitor {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("um valor JSON")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Node, E> {
        Ok(Node::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Node, E> {
        Ok(Node::Number(v as f64))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Node, E> {
        Ok(Node::Number(v as f64))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Node, E> {
        Ok(Node::Number(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Node, E> {
        Ok(Node::Text(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Node, E> {
        Ok(Node::Text(v))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Node, E> {
        Ok(Node::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<Node, E> {
        Ok(Node::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = seq.next_element()? {
            out.push(v);
        }
        Ok(Node::List(out))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
        let mut out = Vec::new();
        while let Some((k, v)) = map.next_entry::<String, Node>()? {
            out.push((k, v));
        }
        Ok(Node::Map(out))
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Node, D::Error> {
        d.deserialize_any(NodeVisitor)
    }
}

/// O custo em pontos por nível de grau do improviso (`improviso.custoPorNivel`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImprovisationCost {
    /// `padrao`: o custo de um parâmetro por grau (1).
    pub standard: f64,
    /// `cura`: o custo por grau da Cura (2).
    pub healing: f64,
}

/// Uma linha do esticar (`esticar.tabela`): quantos níveis acima do nível da
/// Arte, e o preço e a Velocidade como o livro os escreve.
#[derive(Debug, Clone, PartialEq)]
pub struct StretchRow {
    /// `acima`: os níveis acima do nível da Arte (0 a 3).
    pub above: f64,
    /// `custo`: o custo do parâmetro ("nível × 2").
    pub cost: String,
    /// `speed`: o fator da Velocidade ("×2").
    pub speed: String,
}

/// O estado do elemento do lado em que a base se abre
/// (`improviso.manifestacao.numeros.estadoNoLado`): o fator de cada estado.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SideState {
    /// `terra`.
    pub earth: f64,
    /// `metal`.
    pub metal: f64,
    /// `ar`.
    pub air: f64,
    /// `nevoa`.
    pub mist: f64,
    /// `padrao`: o fator dos outros estados.
    pub default: f64,
}

/// Os números da manifestação em arco (`improviso.manifestacao.numeros`).
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestationNumbers {
    /// `aberturas`: as aberturas do arco, em graus.
    pub openings: Vec<f64>,
    /// `aberturaPadrao`: a abertura que vale sem escolha, em graus.
    pub default_opening: f64,
    /// `fatiasMinimo`: o mínimo de fatias.
    pub min_slices: f64,
    /// `fatiasPorNivel`: as fatias que cada nível acrescenta.
    pub slices_per_level: f64,
    /// `estadoNoLado`.
    pub side_state: SideState,
    /// `alturaMinimaBase`: a menor altura da base, em metros.
    pub min_base_height: f64,
}

/// Os números do desvio de uma área (`tempoDaArte.area.desvio.numeros`).
#[derive(Debug, Clone, PartialEq)]
pub struct DetourNumbers {
    /// `ticksPorMetro`: o preço em Ticks de quem desvia fora da vez.
    pub ticks_per_meter: f64,
    /// `metrosMinimo`: o mínimo de metros do desvio.
    pub min_meters: f64,
    /// `difBase`: a Dificuldade de base.
    pub base_difficulty: f64,
    /// `difPorMetro`: a Dificuldade que cada metro soma.
    pub difficulty_per_meter: f64,
    /// `fatorDanoNenhum`: o fator quando o desvio não livra de dano algum.
    pub no_damage_factor: f64,
    /// `bonusIdentificou`: o bônus de quem identificou a Arte.
    pub identified_bonus: f64,
    /// `bonusIdentificouMargem`: o bônus de quem identificou, pela margem.
    pub identified_margin_bonus: f64,
    /// `fracaoMetade`: a fração do "meio" (0,5).
    pub half_fraction: f64,
}

/// O número no caminho: `Ok(None)` se o caminho não existe, erro se existe e
/// não é número.
fn opt_number(tree: &Node, path: &str) -> Result<Option<f64>, String> {
    match tree.get(path) {
        None => Ok(None),
        Some(Node::Number(n)) => Ok(Some(*n)),
        Some(_) => Err(format!("arcano.{path}: esperava um número")),
    }
}

/// O número no caminho, obrigatório.
fn need_number(tree: &Node, path: &str) -> Result<f64, String> {
    opt_number(tree, path)?.ok_or_else(|| format!("arcano.{path}: falta"))
}

fn need_text(node: &Node, key: &str, path: &str) -> Result<String, String> {
    node.entry(key)
        .and_then(Node::as_text)
        .map(str::to_owned)
        .ok_or_else(|| format!("arcano.{path}.{key}: esperava um texto"))
}

/// Os números tipados que saem da árvore.
pub(crate) struct Typed {
    pub improvisation_cost: Option<ImprovisationCost>,
    pub healing_cost_per_level: Option<f64>,
    pub elemental_source_discount: Option<f64>,
    pub stretch_table: Vec<StretchRow>,
    pub manifestation: Option<ManifestationNumbers>,
    pub detour: Option<DetourNumbers>,
}

pub(crate) fn typed(tree: &Node) -> Result<Typed, String> {
    let improvisation_cost = match tree.get("improviso.custoPorNivel") {
        None => None,
        Some(_) => Some(ImprovisationCost {
            standard: need_number(tree, "improviso.custoPorNivel.padrao")?,
            healing: need_number(tree, "improviso.custoPorNivel.cura")?,
        }),
    };
    let mut stretch_table = Vec::new();
    if let Some(rows) = tree.get("esticar.tabela") {
        let rows = rows
            .as_list()
            .ok_or("arcano.esticar.tabela: esperava uma lista")?;
        for (i, row) in rows.iter().enumerate() {
            let at = format!("esticar.tabela[{i}]");
            stretch_table.push(StretchRow {
                above: row
                    .entry("acima")
                    .and_then(Node::as_number)
                    .ok_or_else(|| format!("arcano.{at}.acima: esperava um número"))?,
                cost: need_text(row, "custo", &at)?,
                speed: need_text(row, "speed", &at)?,
            });
        }
    }
    let manifestation = match tree.get("improviso.manifestacao.numeros") {
        None => None,
        Some(_) => {
            let p = "improviso.manifestacao.numeros";
            let n = |k: &str| need_number(tree, &format!("{p}.{k}"));
            let openings = tree
                .list(&format!("{p}.aberturas"))
                .ok_or("arcano.improviso.manifestacao.numeros.aberturas: esperava uma lista")?
                .iter()
                .map(|x| {
                    x.as_number()
                        .ok_or("arcano.improviso.manifestacao.numeros.aberturas: esperava números")
                })
                .collect::<Result<Vec<_>, _>>()?;
            Some(ManifestationNumbers {
                openings,
                default_opening: n("aberturaPadrao")?,
                min_slices: n("fatiasMinimo")?,
                slices_per_level: n("fatiasPorNivel")?,
                side_state: SideState {
                    earth: n("estadoNoLado.terra")?,
                    metal: n("estadoNoLado.metal")?,
                    air: n("estadoNoLado.ar")?,
                    mist: n("estadoNoLado.nevoa")?,
                    default: n("estadoNoLado.padrao")?,
                },
                min_base_height: n("alturaMinimaBase")?,
            })
        }
    };
    let detour = match tree.get("tempoDaArte.area.desvio.numeros") {
        None => None,
        Some(_) => {
            let p = "tempoDaArte.area.desvio.numeros";
            let n = |k: &str| need_number(tree, &format!("{p}.{k}"));
            Some(DetourNumbers {
                ticks_per_meter: n("ticksPorMetro")?,
                min_meters: n("metrosMinimo")?,
                base_difficulty: n("difBase")?,
                difficulty_per_meter: n("difPorMetro")?,
                no_damage_factor: n("fatorDanoNenhum")?,
                identified_bonus: n("bonusIdentificou")?,
                identified_margin_bonus: n("bonusIdentificouMargem")?,
                half_fraction: n("fracaoMetade")?,
            })
        }
    };
    Ok(Typed {
        improvisation_cost,
        healing_cost_per_level: opt_number(tree, "cura.custoPorNivel")?,
        elemental_source_discount: opt_number(tree, "fonteElemental.desconto")?,
        stretch_table,
        manifestation,
        detour,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Node {
        crate::json::parse(json).unwrap()
    }

    #[test]
    fn reads_by_path_and_keeps_the_text_untouched() {
        let n = parse(
            r#"{"a":{"b":[{"t":"  com espaço   e `crase`  "},{"n":2.5,"f":false,"z":null}]}}"#,
        );
        assert_eq!(n.text("a.b.0.t"), Some("  com espaço \u{a0} e `crase`  "));
        assert_eq!(n.number("a.b.1.n"), Some(2.5));
        assert_eq!(n.get("a.b.1.f").and_then(Node::as_bool), Some(false));
        assert!(n.get("a.b.1.z").unwrap().is_null());
        assert_eq!(n.list("a.b").map(<[Node]>::len), Some(2));
        assert_eq!(n.get("").unwrap(), &n);
    }

    #[test]
    fn a_missing_path_is_none() {
        let n = parse(r#"{"a":{"b":[1]},"s":"x"}"#);
        for p in ["z", "a.z", "a.b.1", "a.b.x", "s.t", "a.b.0.c", "a."] {
            assert_eq!(n.get(p), None, "{p}");
        }
        // o tipo errado também é None
        assert_eq!(n.text("a"), None);
        assert_eq!(n.number("s"), None);
        assert_eq!(n.list("a"), None);
    }

    #[test]
    fn keeps_the_file_order_of_the_keys() {
        let n = parse(r#"{"z":1,"a":2,"m":3}"#);
        let keys: Vec<&str> = n.map("").unwrap().iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["z", "a", "m"]);
    }

    #[test]
    fn typed_numbers_are_checked() {
        let bad = parse(r#"{"improviso":{"custoPorNivel":{"padrao":"1","cura":2}}}"#);
        assert!(typed(&bad).is_err());
        let ok = parse(r#"{"improviso":{"custoPorNivel":{"padrao":1,"cura":2}}}"#);
        let t = typed(&ok).unwrap();
        assert_eq!(
            t.improvisation_cost,
            Some(ImprovisationCost {
                standard: 1.0,
                healing: 2.0
            })
        );
        assert_eq!(t.elemental_source_discount, None);
        assert!(t.stretch_table.is_empty());
    }
}
