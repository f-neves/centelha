//! O gerador do site estático (feature `ssr`): lê `content/`, desenha cada
//! página em HTML e grava `dist/`.
//!
//! O WebAssembly (as ilhas) e o script de carga entram em `dist/pkg/`, gravados
//! pelo `cargo xtask site` depois do gerador.

mod backdrop;
pub mod equipment;
pub mod links;
pub mod markdown;
pub mod nav;
pub mod pages;
pub mod refs;
mod slug;
mod slug_table;
pub mod text;
mod typography;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use centelha_data::{Chapter, DataError, GlossaryEntry, load_chapters, load_glossary};
use hydration_context::SsrSharedContext;
use leptos::config::LeptosOptions;
use leptos::hydration::HydrationScripts;
use leptos::prelude::*;
use leptos::reactive::owner::Owner;

use markdown::{Diagrams, DiagramsError, Markdown};
use pages::{
    ChapterPage, EquipmentPage, GlossaryPage, HomeCounts, HomePage, IslandsTestPage, Layout,
    Neighbor, NotFoundPage, PageInfo,
};

/// O caminho base de quem não diz outro (a prévia no GitHub Pages).
pub const DEFAULT_BASE: &str = "/centelha/";

/// O nome dos arquivos do WebAssembly e do script de cola em `dist/pkg/`.
pub const PKG_NAME: &str = "centelha_web";

/// A folha de estilo do site, gravada em `dist/assets/site.css`.
const STYLESHEET: &str = include_str!("../../style/site.css");

/// As fontes, gravadas em `dist/assets/fonts/` byte a byte (a folha de estilo
/// aponta para elas com caminho relativo, que já leva o caminho base).
const FONTS: &[(&str, &[u8])] = &[
    (
        "cinzel-latin-wght-normal.woff2",
        include_bytes!("../../fonts/cinzel/cinzel-latin-wght-normal.woff2"),
    ),
    (
        "eb-garamond-latin-400-normal.woff2",
        include_bytes!("../../fonts/eb-garamond/eb-garamond-latin-400-normal.woff2"),
    ),
    (
        "eb-garamond-latin-400-italic.woff2",
        include_bytes!("../../fonts/eb-garamond/eb-garamond-latin-400-italic.woff2"),
    ),
    (
        "eb-garamond-latin-600-normal.woff2",
        include_bytes!("../../fonts/eb-garamond/eb-garamond-latin-600-normal.woff2"),
    ),
];

/// A fonte pré-carregada: a do texto corrido, que aparece na primeira tela.
/// As outras carregam quando a página pede.
const PRELOADED_FONT: &str = "eb-garamond-latin-400-normal.woff2";

/// O script do `<head>`, antes da primeira pintura: tema, barra lateral
/// recolhida e preferências de diagramação (veja o comentário em `head.js`).
const HEAD_SCRIPT: &str = include_str!("head.js");

/// O caminho base com `/` no começo e no fim (`centelha` vira `/centelha/`;
/// vazio vira `/`).
pub fn normalize_base(raw: &str) -> String {
    let inner = raw.trim().trim_matches('/');
    if inner.is_empty() {
        "/".to_string()
    } else {
        format!("/{inner}/")
    }
}

/// O que pode dar errado ao gerar o site.
#[derive(Debug)]
pub enum SiteError {
    /// Um capítulo não se leu.
    Data(DataError),
    /// `diagramas.json` não se leu.
    Diagrams(DiagramsError),
    /// O catálogo de equipamento não se montou.
    Equipment(centelha_engine::sheet::SheetError),
    /// Um arquivo não se leu ou não se gravou.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for SiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SiteError::Data(e) => write!(f, "{e}"),
            SiteError::Diagrams(e) => write!(f, "{e}"),
            SiteError::Equipment(e) => write!(f, "{e}"),
            SiteError::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for SiteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SiteError::Data(e) => Some(e),
            SiteError::Diagrams(e) => Some(e),
            SiteError::Equipment(e) => Some(e),
            SiteError::Io { source, .. } => Some(source),
        }
    }
}

impl From<DataError> for SiteError {
    fn from(e: DataError) -> Self {
        SiteError::Data(e)
    }
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> SiteError + '_ {
    move |e| SiteError::Io {
        path: path.to_path_buf(),
        source: e,
    }
}

/// Uma página pronta: o caminho dentro de `dist/` e o HTML.
#[derive(Debug, Clone)]
pub struct Page {
    pub path: String,
    pub html: String,
}

/// O site lido de `content/`.
pub struct Site {
    base: String,
    chapters: Vec<Chapter>,
    glossary: Vec<GlossaryEntry>,
    /// O glossário na ordem do arquivo (a ordem de desempate da marcação).
    glossary_source: Vec<GlossaryEntry>,
    counts: HomeCounts,
    scales: HashMap<String, leptos::serde_json::Value>,
    equipment: equipment::Equipment,
    diagrams: Diagrams,
    content_dir: PathBuf,
}

impl Site {
    /// Lê os capítulos, o glossário, os diagramas e os números da página
    /// inicial. `base` passa por [`normalize_base`].
    pub fn load(content_dir: &Path, base: &str) -> Result<Site, SiteError> {
        let chapters = load_chapters(content_dir)?;
        let glossary_source = load_glossary(content_dir)?;
        let mut glossary = glossary_source.clone();
        glossary.sort_by(|a, b| centelha_engine::collate::compare(&a.term, &b.term));
        let diagrams_path = content_dir.join("data").join("diagramas.json");
        let json = std::fs::read_to_string(&diagrams_path).map_err(io_error(&diagrams_path))?;
        let diagrams = Diagrams::parse(&json).map_err(SiteError::Diagrams)?;
        Ok(Site {
            base: normalize_base(base),
            chapters,
            glossary,
            glossary_source,
            counts: home_counts(content_dir)?,
            scales: glossary_scales(content_dir)?,
            equipment: equipment::load(content_dir, &skill_names(content_dir)?)?,
            diagrams,
            content_dir: content_dir.to_path_buf(),
        })
    }

    /// Os termos do glossário, na ordem da página.
    pub fn glossary(&self) -> &[GlossaryEntry] {
        &self.glossary
    }

    /// Os números da página inicial.
    pub fn counts(&self) -> HomeCounts {
        self.counts
    }

    /// O caminho base, normalizado.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Os capítulos, em ordem.
    pub fn chapters(&self) -> &[Chapter] {
        &self.chapters
    }

    /// A conversão de Markdown deste site.
    pub fn markdown(&self) -> Markdown<'_> {
        Markdown {
            base: &self.base,
            diagrams: &self.diagrams,
        }
    }

    /// Todas as páginas: a inicial, uma por capítulo e o glossário.
    pub fn pages(&self) -> Vec<Page> {
        let mut pages = Vec::with_capacity(self.chapters.len() + 2);
        let base = self.base.clone();
        let counts = self.counts;
        let home = PageInfo {
            path: String::new(),
            title: "Início".to_string(),
            wide: true,
            toc: Vec::new(),
            refs: false,
        };
        pages.push(Page {
            path: "index.html".to_string(),
            html: self.document(home, None, move || {
                view! { <HomePage base counts/> }.into_any()
            }),
        });
        for (i, chapter) in self.chapters.iter().enumerate() {
            pages.push(self.chapter_page(i, chapter));
        }
        let entries = self.glossary.clone();
        let glossary = PageInfo {
            path: "glossario".to_string(),
            title: "Glossário".to_string(),
            wide: false,
            toc: Vec::new(),
            refs: false,
        };
        pages.push(Page {
            path: "glossario/index.html".to_string(),
            html: self.document(
                glossary,
                Some("Os termos-chave do sistema, com sinônimos e definições."),
                move || view! { <GlossaryPage entries/> }.into_any(),
            ),
        });
        let equipment = self.equipment.clone();
        let base = self.base.clone();
        let equipment_page = PageInfo {
            path: "equipamentos".to_string(),
            title: "Equipamentos".to_string(),
            wide: true,
            toc: Vec::new(),
            refs: false,
        };
        pages.push(Page {
            path: "equipamentos/index.html".to_string(),
            html: self.document(
                equipment_page,
                Some("Catálogo filtrável de armas, armaduras e escudos, com dano, Absorção e os trade-offs."),
                move || view! { <EquipmentPage base equipment/> }.into_any(),
            ),
        });
        // A página de "não encontrada", na raiz de `dist/` (o GitHub Pages a
        // serve para qualquer endereço sem página). Todos os endereços dela
        // são absolutos, com o caminho base, para valer de qualquer pasta.
        let base = self.base.clone();
        let not_found = PageInfo {
            path: "404".to_string(),
            title: "Página não encontrada".to_string(),
            wide: false,
            toc: Vec::new(),
            refs: false,
        };
        pages.push(Page {
            path: "404.html".to_string(),
            html: self.document(not_found, None, move || {
                view! { <NotFoundPage base/> }.into_any()
            }),
        });
        pages
    }

    fn chapter_page(&self, i: usize, chapter: &Chapter) -> Page {
        let prev = i
            .checked_sub(1)
            .map(|p| Neighbor::new(&self.base, chapter, &self.chapters[p]));
        let next = self
            .chapters
            .get(i + 1)
            .map(|n| Neighbor::new(&self.base, chapter, n));
        let markdown = self.markdown();
        let body_html = markdown.to_html(&chapter.body);
        let numeral = chapter.numeral.clone();
        let title = chapter.title.clone();
        let info = PageInfo {
            path: format!("regras/{}", chapter.slug),
            title: chapter.title.clone(),
            wide: false,
            toc: markdown.headings(&chapter.body),
            refs: true,
        };
        Page {
            path: format!("regras/{}/index.html", chapter.slug),
            html: self.document(info, Some(&chapter.summary), move || {
                view! { <ChapterPage numeral title body_html prev next/> }.into_any()
            }),
        }
    }

    /// O documento inteiro de uma página. O corpo é desenhado num contexto de
    /// ilhas: só o que é `#[island]` leva marcas de hidratação.
    fn document(
        &self,
        page: PageInfo,
        description: Option<&str>,
        content: impl FnOnce() -> AnyView + Send + 'static,
    ) -> String {
        let base = self.base.clone();
        let root = base.trim_end_matches('/').to_string();
        let full_title = if page.path.is_empty() {
            "Centelha".to_string()
        } else {
            format!("{} · Centelha", page.title)
        };
        let has_refs = page.refs;
        let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new_islands())));
        let (scripts, mut body) = owner.with(move || {
            let options = LeptosOptions::builder()
                .output_name(PKG_NAME)
                .site_pkg_dir("pkg")
                .build();
            let scripts = view! { <HydrationScripts options islands=true root/> }.to_html();
            let body = view! { <Layout base page>{content()}</Layout> }.to_html();
            (scripts, body)
        });
        if has_refs {
            body = self.mark_refs(&body);
        }
        body = body.replacen(pages::REF_INDEX_SLOT, &self.ref_index_json(), 1);
        let description =
            description.unwrap_or("Centelha: sistema de RPG de mesa em d6, do mortal ao semideus.");
        let b = &self.base;
        format!(
            "<!DOCTYPE html>\n\
<html lang=\"pt-BR\" data-theme=\"escuro\">\n\
<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{title}</title>\n\
<meta name=\"description\" content=\"{description}\">\n\
<link rel=\"icon\" href=\"{b}favicon.ico\" sizes=\"any\">\n\
<link rel=\"icon\" href=\"{b}favicon.svg\" type=\"image/svg+xml\">\n\
<link rel=\"icon\" href=\"{b}favicon-32.png\" type=\"image/png\" sizes=\"32x32\">\n\
<link rel=\"icon\" href=\"{b}favicon-16.png\" type=\"image/png\" sizes=\"16x16\">\n\
<link rel=\"manifest\" href=\"{b}manifest.webmanifest\">\n\
<link rel=\"apple-touch-icon\" href=\"{b}icon-192-v2.png\">\n\
<meta name=\"theme-color\" content=\"#0B3B47\">\n\
<script>{HEAD_SCRIPT}</script>\n\
<link rel=\"preload\" href=\"{b}assets/fonts/{PRELOADED_FONT}\" as=\"font\" type=\"font/woff2\" crossorigin>\n\
<link rel=\"stylesheet\" href=\"{b}assets/site.css\">\n\
{scripts}\n\
</head>\n\
<body>\n{body}\n</body>\n\
</html>\n",
            title = escape_html(&full_title),
            description = escape_html(description),
        )
    }

    /// Grava as páginas, a folha de estilo, as fontes e os ícones em `out_dir`. Devolve
    /// as páginas gravadas.
    pub fn write(&self, out_dir: &Path) -> Result<Vec<Page>, SiteError> {
        let pages = self.pages();
        for page in &pages {
            let path = out_dir.join(&page.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(io_error(parent))?;
            }
            std::fs::write(&path, &page.html).map_err(io_error(&path))?;
        }
        let assets = out_dir.join("assets");
        std::fs::create_dir_all(&assets).map_err(io_error(&assets))?;
        let css = assets.join("site.css");
        std::fs::write(&css, STYLESHEET).map_err(io_error(&css))?;
        let fonts = assets.join("fonts");
        std::fs::create_dir_all(&fonts).map_err(io_error(&fonts))?;
        for (name, bytes) in FONTS {
            let path = fonts.join(name);
            std::fs::write(&path, bytes).map_err(io_error(&path))?;
        }
        self.copy_public_files(out_dir)?;
        Ok(pages)
    }

    /// A marcação das referências do glossário.
    pub fn refs(&self) -> refs::Refs<'_> {
        refs::Refs::new(&self.base, &self.glossary_source)
    }

    /// Marca os termos do glossário no `<main>` da página pronta (como o site
    /// atual, que varre o `<main>` inteiro: a trilha e o numeral do capítulo
    /// entram, e a ordem do "uma vez por bloco" fica a mesma) e grava os dados
    /// do cartão no `<script>` que a casca deixou vazio.
    fn mark_refs(&self, body: &str) -> String {
        let (Some(start), Some(end)) = (body.find("<main"), body.find("</main>")) else {
            return body.to_string();
        };
        let marked = self.refs().mark(&body[start..end]);
        let json = self.refs_json(&marked).unwrap_or_else(|| "[]".to_string());
        let mut out = String::with_capacity(body.len() + marked.html.len());
        out.push_str(&body[..start]);
        out.push_str(&marked.html);
        out.push_str(&body[end..].replacen(pages::REF_DATA_SLOT, &json, 1));
        out
    }

    /// O índice dos termos para o painel dos fixados (`crate::bookmarks`): o
    /// glossário inteiro, com nome, resumo e endereço.
    fn ref_index_json(&self) -> String {
        let items: Vec<leptos::serde_json::Value> = self
            .glossary
            .iter()
            .map(|g| {
                leptos::serde_json::json!({
                    "id": g.id,
                    "kind": "termo",
                    "name": g.term,
                    "summary": g.definition,
                    "url": nav::url(&self.base, &format!("glossario#{}", g.id)),
                })
            })
            .collect();
        leptos::serde_json::to_string(&items)
            .unwrap_or_else(|_| "[]".to_string())
            .replace("</", "<\\/")
    }

    /// Uma página de teste das ilhas que ainda não têm página portada (o
    /// cartão das formas e o voltar ao topo, das páginas das Artes). Não vai
    /// para o `dist/`: o binário só a grava com `--test-page`.
    pub fn test_page(&self) -> Page {
        let info = PageInfo {
            path: "teste-ilhas".to_string(),
            title: "Teste das ilhas".to_string(),
            wide: false,
            toc: Vec::new(),
            refs: false,
        };
        Page {
            path: "teste-ilhas/index.html".to_string(),
            html: self.document(info, None, move || view! { <IslandsTestPage/> }.into_any()),
        }
    }

    /// Os dados do cartão para os termos da página: os marcados e os que um
    /// link da página aponta (`#id` de um termo), num JSON para o
    /// `<script type="application/json">` que a ilha lê. `None` sem termo.
    fn refs_json(&self, marked: &refs::Marked) -> Option<String> {
        let mut ids = marked.ids();
        for href in marked
            .html
            .split("href=\"")
            .skip(1)
            .filter_map(|r| r.split('"').next())
        {
            if let Some((_, fragment)) = href.split_once('#')
                && self.glossary.iter().any(|g| g.id == fragment)
            {
                ids.insert(fragment.to_string());
            }
        }
        if ids.is_empty() {
            return None;
        }
        let items: Vec<leptos::serde_json::Value> = self
            .glossary
            .iter()
            .filter(|g| ids.contains(&g.id))
            .map(|g| {
                let mut item = leptos::serde_json::json!({
                    "id": g.id,
                    "kind": "termo",
                    "name": g.term,
                    "url": nav::url(&self.base, &format!("glossario#{}", g.id)),
                    "summary": g.definition,
                });
                if let Some(scale) = self.scales.get(&g.id) {
                    item["levels"] = scale.clone();
                }
                item
            })
            .collect();
        let json = leptos::serde_json::to_string(&items).ok()?;
        // Dentro de <script>, `</` fecharia a etiqueta.
        Some(json.replace("</", "<\\/"))
    }

    /// Os links internos das páginas que não têm destino no site gerado (as
    /// páginas ainda não portadas), com quantas vezes cada um aparece.
    pub fn missing_links(&self, pages: &[Page]) -> Result<Vec<links::MissingLink>, SiteError> {
        let mut files: Vec<String> = pages.iter().map(|p| p.path.clone()).collect();
        files.push("assets/site.css".to_string());
        files.extend(FONTS.iter().map(|(name, _)| format!("assets/fonts/{name}")));
        let public = self.content_dir.join("public");
        for entry in std::fs::read_dir(&public).map_err(io_error(&public))? {
            let entry = entry.map_err(io_error(&public))?;
            if entry.path().is_file() {
                files.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        Ok(links::missing_links(&self.base, pages, &files))
    }

    /// Copia os arquivos soltos de `content/public/` (ícones e manifesto)
    /// como estão, byte a byte. As pastas (imagens do bestiário) entram com as
    /// páginas que as usam.
    fn copy_public_files(&self, out_dir: &Path) -> Result<(), SiteError> {
        let public = self.content_dir.join("public");
        for entry in std::fs::read_dir(&public).map_err(io_error(&public))? {
            let entry = entry.map_err(io_error(&public))?;
            let path = entry.path();
            if path.is_file() {
                let target = out_dir.join(entry.file_name());
                std::fs::copy(&path, &target).map_err(io_error(&target))?;
            }
        }
        Ok(())
    }
}

/// As escalas que o cartão do glossário mostra em dois termos (Centelha e
/// Força de Vontade), como no índice de referências do site atual
/// (`src/pages/ref-index.json.ts`): `escalaCentelha` e `escalaVontade` de
/// `regras.json`.
///
/// Provisório, como a contagem da página inicial: o `centelha-data` ainda não
/// expõe essas escalas.
/// Os nomes das Habilidades por id (a coluna Habilidade de Equipamentos).
///
/// Provisório, como as escalas e os contadores: o `centelha-data` ainda não
/// tem o módulo das Habilidades. Lê só `id` e `nome` de `habilidades.json`.
fn skill_names(content_dir: &Path) -> Result<Vec<(String, String)>, SiteError> {
    let path = content_dir.join("data").join("habilidades.json");
    let text = std::fs::read_to_string(&path).map_err(io_error(&path))?;
    let list: Vec<leptos::serde_json::Value> =
        leptos::serde_json::from_str(&text).map_err(|e| {
            SiteError::Data(DataError::Json {
                file: Some("data/habilidades.json".to_string()),
                path: String::new(),
                reason: e.to_string(),
            })
        })?;
    Ok(list
        .iter()
        .filter_map(|h| {
            Some((
                h.get("id")?.as_str()?.to_string(),
                h.get("nome")?.as_str()?.to_string(),
            ))
        })
        .collect())
}

fn glossary_scales(
    content_dir: &Path,
) -> Result<HashMap<String, leptos::serde_json::Value>, SiteError> {
    let path = content_dir.join("data").join("regras.json");
    let text = std::fs::read_to_string(&path).map_err(io_error(&path))?;
    let rules: leptos::serde_json::Value = leptos::serde_json::from_str(&text).map_err(|e| {
        SiteError::Data(DataError::Json {
            file: Some("data/regras.json".to_string()),
            path: String::new(),
            reason: e.to_string(),
        })
    })?;
    let mut scales = HashMap::new();
    for (id, key) in [("centelha", "escalaCentelha"), ("vontade", "escalaVontade")] {
        if let Some(scale) = rules.get(key) {
            scales.insert(id.to_string(), scale.clone());
        }
    }
    Ok(scales)
}

/// Os números da página inicial, contados como o site atual conta: as
/// Técnicas de `tecnicas.json`, os Caminhos distintos delas (as Proezas) e as
/// Artes de `artes.json`.
///
/// Por enquanto a contagem lê os dois arquivos aqui, só pelo tamanho e pelo
/// campo `caminho`: o `centelha-data` ainda não tem os módulos de Técnicas e
/// Artes (entram na fase 2). Quando entrarem, a contagem passa a usá-los.
fn home_counts(content_dir: &Path) -> Result<HomeCounts, SiteError> {
    let read = |name: &str| -> Result<Vec<leptos::serde_json::Value>, SiteError> {
        let path = content_dir.join("data").join(name);
        let text = std::fs::read_to_string(&path).map_err(io_error(&path))?;
        leptos::serde_json::from_str(&text).map_err(|e| {
            SiteError::Data(DataError::Json {
                file: Some(format!("data/{name}")),
                path: String::new(),
                reason: e.to_string(),
            })
        })
    };
    let techniques = read("tecnicas.json")?;
    let feats: std::collections::BTreeSet<String> = techniques
        .iter()
        .filter_map(|t| match &t["caminho"] {
            leptos::serde_json::Value::String(id) => Some(id.clone()),
            other => other["id"].as_str().map(str::to_string),
        })
        .collect();
    Ok(HomeCounts {
        feats: feats.len(),
        techniques: techniques.len(),
        arts: read("artes.json")?.len(),
    })
}

/// Escapa texto para dentro de HTML (conteúdo ou atributo entre aspas).
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_is_normalized() {
        assert_eq!(normalize_base("/centelha/"), "/centelha/");
        assert_eq!(normalize_base("centelha"), "/centelha/");
        assert_eq!(normalize_base("/"), "/");
        assert_eq!(normalize_base(""), "/");
    }
}
