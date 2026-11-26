//! Os capítulos do livro: `content/chapters/*.md`.
//!
//! Cada arquivo abre com um cabeçalho entre duas linhas `---`, uma linha por
//! campo, `chave: valor`. O valor é texto entre aspas duplas ou, na `ordem`,
//! um número inteiro. Não é YAML completo: é o pedaço de YAML que os capítulos
//! usam, e o que fugir disso dá erro, para a fonte não mudar de forma sem
//! ninguém ver. Dentro das aspas valem só os escapes `\"` e `\\`.

use crate::DataError;

/// Um capítulo do livro (uma página).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    /// O nome do arquivo sem `.md`; é também o endereço da página.
    pub slug: String,
    /// `ordem`: a posição do capítulo no livro.
    pub order: i64,
    /// `numeral`: o algarismo romano do capítulo.
    pub numeral: String,
    /// `titulo`.
    pub title: String,
    /// `resumo`.
    pub summary: String,
    /// `capitulo`: o nome do capítulo de várias páginas, quando a primeira
    /// página tem outro título.
    pub chapter_name: Option<String>,
    /// O Markdown sem o cabeçalho.
    pub body: String,
}

/// Lê um capítulo a partir do texto do arquivo. `slug` é o nome do arquivo
/// sem `.md`.
pub fn parse_chapter(slug: &str, text: &str) -> Result<Chapter, DataError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let (header, body) = split_header(text)?;

    let mut order = None;
    let mut numeral = None;
    let mut title = None;
    let mut summary = None;
    let mut chapter_name = None;

    for (n, line) in header.lines().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let place = format!("cabeçalho, linha {}", n + 2);
        let Some((key, value)) = line.split_once(':') else {
            return Err(DataError::invalid(
                place,
                format!("linha sem `chave: valor`: {line:?}"),
            ));
        };
        let key = key.trim();
        let value = value.trim();
        let slot = match key {
            "ordem" => {
                let parsed = value.parse::<i64>().map_err(|_| {
                    DataError::invalid(
                        place.clone(),
                        format!("`ordem` tem de ser um número inteiro, veio {value:?}"),
                    )
                })?;
                set_once(&mut order, parsed, key, &place)?;
                continue;
            }
            "numeral" => &mut numeral,
            "titulo" => &mut title,
            "resumo" => &mut summary,
            "capitulo" => &mut chapter_name,
            other => {
                return Err(DataError::invalid(
                    place,
                    format!("chave desconhecida: {other:?}"),
                ));
            }
        };
        let parsed = quoted(value).map_err(|reason| DataError::invalid(place.clone(), reason))?;
        set_once(slot, parsed, key, &place)?;
    }

    let missing = |key: &str| DataError::invalid("cabeçalho", format!("falta o campo `{key}`"));
    Ok(Chapter {
        slug: slug.to_string(),
        order: order.ok_or_else(|| missing("ordem"))?,
        numeral: numeral.ok_or_else(|| missing("numeral"))?,
        title: title.ok_or_else(|| missing("titulo"))?,
        summary: summary.ok_or_else(|| missing("resumo"))?,
        chapter_name,
        body: body.to_string(),
    })
}

/// Lê todos os capítulos de `<content_dir>/chapters/`, em ordem de `ordem`
/// (só no nativo). A fonte não proíbe `ordem` repetida: o empate se desfaz
/// pelo `slug`, para a ordem não depender da listagem da pasta.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_chapters(content_dir: &std::path::Path) -> Result<Vec<Chapter>, DataError> {
    let dir = content_dir.join("chapters");
    let io = |file: &std::path::Path, e: std::io::Error| DataError::Io {
        file: file.display().to_string(),
        reason: e.to_string(),
    };
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| io(&dir, e))? {
        let path = entry.map_err(|e| io(&dir, e))?.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            paths.push(path);
        }
    }
    paths.sort();

    let mut chapters = Vec::with_capacity(paths.len());
    for path in paths {
        let text = std::fs::read_to_string(&path).map_err(|e| io(&path, e))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let slug = name.trim_end_matches(".md");
        let chapter =
            parse_chapter(slug, &text).map_err(|e| e.in_file(format!("chapters/{name}")))?;
        chapters.push(chapter);
    }
    chapters.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.slug.cmp(&b.slug)));
    Ok(chapters)
}

/// Separa o cabeçalho (sem as linhas `---`) do corpo.
fn split_header(text: &str) -> Result<(&str, &str), DataError> {
    let after_open = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
        .ok_or_else(|| DataError::invalid("cabeçalho", "o arquivo não abre com `---`"))?;
    let mut offset = 0;
    for line in after_open.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        if bare == "---" {
            let header = &after_open[..offset];
            let body = &after_open[offset + line.len()..];
            return Ok((header, body));
        }
        offset += line.len();
    }
    Err(DataError::invalid(
        "cabeçalho",
        "o cabeçalho não fecha com `---`",
    ))
}

/// O texto de um valor entre aspas duplas.
fn quoted(value: &str) -> Result<String, String> {
    let inner = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .filter(|_| value.len() >= 2)
        .ok_or_else(|| format!("o valor tem de vir entre aspas duplas, veio {value:?}"))?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                other => {
                    return Err(format!(
                        "escape não aceito: \\{}",
                        other.map(String::from).unwrap_or_default()
                    ));
                }
            },
            '"' => return Err(format!("aspas sem escape dentro do valor: {value:?}")),
            c => out.push(c),
        }
    }
    Ok(out)
}

fn set_once<T>(slot: &mut Option<T>, value: T, key: &str, place: &str) -> Result<(), DataError> {
    if slot.is_some() {
        return Err(DataError::invalid(place, format!("campo `{key}` repetido")));
    }
    *slot = Some(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "---\nordem: 10\nnumeral: \"VIII\"\ntitulo: \"A Régua Comum\"\ncapitulo: \"Ações & Sistema\"\nresumo: \"O motor \\\"comum\\\".\"\n---\n# Corpo\n\nTexto.\n";

    #[test]
    fn reads_all_fields() {
        let c = parse_chapter("acoes-e-sistema", GOOD).unwrap();
        assert_eq!(c.slug, "acoes-e-sistema");
        assert_eq!(c.order, 10);
        assert_eq!(c.numeral, "VIII");
        assert_eq!(c.title, "A Régua Comum");
        assert_eq!(c.chapter_name.as_deref(), Some("Ações & Sistema"));
        assert_eq!(c.summary, "O motor \"comum\".");
        assert_eq!(c.body, "# Corpo\n\nTexto.\n");
    }

    #[test]
    fn accepts_crlf() {
        let text = GOOD.replace('\n', "\r\n");
        let c = parse_chapter("x", &text).unwrap();
        assert_eq!(c.title, "A Régua Comum");
        assert_eq!(c.body, "# Corpo\r\n\r\nTexto.\r\n");
    }

    fn reason(text: &str) -> String {
        match parse_chapter("x", text) {
            Err(DataError::Invalid { reason, .. }) => reason,
            other => panic!("esperava Invalid, veio {other:?}"),
        }
    }

    #[test]
    fn broken_headers_are_invalid() {
        let base = "ordem: 1\nnumeral: \"I\"\ntitulo: \"T\"\nresumo: \"R\"\n";
        let cases = [
            ("sem abertura", format!("{base}---\nx")),
            ("sem fechamento", format!("---\n{base}x")),
            ("linha sem dois-pontos", format!("---\n{base}solta\n---\n")),
            (
                "chave desconhecida",
                format!("---\n{base}autor: \"A\"\n---\n"),
            ),
            (
                "valor sem aspas",
                "---\nordem: 1\nnumeral: I\ntitulo: \"T\"\nresumo: \"R\"\n---\n".to_string(),
            ),
            (
                "ordem não inteira",
                "---\nordem: \"1\"\nnumeral: \"I\"\ntitulo: \"T\"\nresumo: \"R\"\n---\n"
                    .to_string(),
            ),
            (
                "ordem fracionária",
                "---\nordem: 1.5\nnumeral: \"I\"\ntitulo: \"T\"\nresumo: \"R\"\n---\n".to_string(),
            ),
            (
                "campo faltando",
                "---\nordem: 1\nnumeral: \"I\"\ntitulo: \"T\"\n---\n".to_string(),
            ),
            ("campo repetido", format!("---\n{base}titulo: \"U\"\n---\n")),
            (
                "aspas abertas",
                "---\nordem: 1\nnumeral: \"I\ntitulo: \"T\"\nresumo: \"R\"\n---\n".to_string(),
            ),
            (
                "escape estranho",
                "---\nordem: 1\nnumeral: \"I\\n\"\ntitulo: \"T\"\nresumo: \"R\"\n---\n".to_string(),
            ),
            (
                "aspas soltas dentro",
                "---\nordem: 1\nnumeral: \"I\"\ntitulo: \"T\"x\"\nresumo: \"R\"\n---\n".to_string(),
            ),
        ];
        for (name, text) in cases {
            let r = reason(&text);
            assert!(!r.is_empty(), "{name}");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn loads_the_28_chapters_in_order() {
        let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let chapters = load_chapters(&content).unwrap();
        assert_eq!(chapters.len(), 28);
        assert!(chapters.windows(2).all(|w| w[0].order < w[1].order));
        assert_eq!(chapters[0].slug, "coracao-do-sistema");
        let with_name: Vec<_> = chapters
            .iter()
            .filter(|c| c.chapter_name.is_some())
            .map(|c| c.slug.as_str())
            .collect();
        assert_eq!(with_name, ["acoes-e-sistema"]);
        assert!(chapters.iter().all(|c| !c.body.starts_with("---")));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn repeated_order_breaks_ties_by_slug() {
        let content =
            std::env::temp_dir().join(format!("centelha-chapters-{}", std::process::id()));
        let dir = content.join("chapters");
        std::fs::create_dir_all(&dir).unwrap();
        let chapter = |order: i64| {
            format!("---\nordem: {order}\nnumeral: \"I\"\ntitulo: \"T\"\nresumo: \"R\"\n---\n")
        };
        // Gravados fora de ordem: `b` e `a` empatam em 2; `c` vem antes.
        for (slug, order) in [("b", 2), ("c", 1), ("a", 2)] {
            std::fs::write(dir.join(format!("{slug}.md")), chapter(order)).unwrap();
        }
        let loaded = load_chapters(&content);
        std::fs::remove_dir_all(&content).unwrap();
        let slugs: Vec<_> = loaded.unwrap().into_iter().map(|c| c.slug).collect();
        assert_eq!(slugs, ["c", "a", "b"]);
    }
}
