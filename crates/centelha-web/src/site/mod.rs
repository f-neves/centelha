//! O gerador do site estático (feature `ssr`): lê `content/`, desenha cada
//! página em HTML e grava `dist/`.
//!
//! O WebAssembly (as ilhas) e o script de carga entram em `dist/pkg/`, gravados
//! pelo `cargo xtask site` depois do gerador.

pub mod markdown;
pub mod pages;
mod slug;
mod slug_table;
pub mod text;
mod typography;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use centelha_data::{Chapter, DataError, load_chapters};
use hydration_context::SsrSharedContext;
use leptos::config::LeptosOptions;
use leptos::hydration::HydrationScripts;
use leptos::prelude::*;
use leptos::reactive::owner::Owner;

use markdown::{Diagrams, Markdown};
use pages::{ChapterPage, HomePage, Layout, Neighbor};

/// O caminho base de quem não diz outro (a prévia no GitHub Pages).
pub const DEFAULT_BASE: &str = "/centelha/";

/// O nome dos arquivos do WebAssembly e do script de cola em `dist/pkg/`.
pub const PKG_NAME: &str = "centelha_web";

/// A folha de estilo do site, gravada em `dist/assets/site.css`.
const STYLESHEET: &str = include_str!("../../style/site.css");

/// Aplica o tema gravado antes da primeira pintura (o padrão é o escuro,
/// que já vem no `<html>` para valer sem JavaScript).
const THEME_SCRIPT: &str = "(function(){try{var t=localStorage.getItem('tema')||'escuro';\
document.documentElement.dataset.theme=t;}catch(e){}})();";

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
    Diagrams(String),
    /// Um arquivo não se leu ou não se gravou.
    Io { path: PathBuf, reason: String },
}

impl fmt::Display for SiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SiteError::Data(e) => write!(f, "{e}"),
            SiteError::Diagrams(e) => write!(f, "{e}"),
            SiteError::Io { path, reason } => write!(f, "{}: {reason}", path.display()),
        }
    }
}

impl std::error::Error for SiteError {}

impl From<DataError> for SiteError {
    fn from(e: DataError) -> Self {
        SiteError::Data(e)
    }
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> SiteError + '_ {
    move |e| SiteError::Io {
        path: path.to_path_buf(),
        reason: e.to_string(),
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
    diagrams: Diagrams,
    content_dir: PathBuf,
}

impl Site {
    /// Lê os capítulos e os diagramas. `base` passa por [`normalize_base`].
    pub fn load(content_dir: &Path, base: &str) -> Result<Site, SiteError> {
        let chapters = load_chapters(content_dir)?;
        let diagrams_path = content_dir.join("data").join("diagramas.json");
        let json = std::fs::read_to_string(&diagrams_path).map_err(io_error(&diagrams_path))?;
        let diagrams = Diagrams::parse(&json).map_err(SiteError::Diagrams)?;
        Ok(Site {
            base: normalize_base(base),
            chapters,
            diagrams,
            content_dir: content_dir.to_path_buf(),
        })
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

    /// Todas as páginas: a inicial e uma por capítulo.
    pub fn pages(&self) -> Vec<Page> {
        let mut pages = Vec::with_capacity(self.chapters.len() + 1);
        let chapters = self.chapters.clone();
        let base = self.base.clone();
        pages.push(Page {
            path: "index.html".to_string(),
            html: self.document("Início", None, move || {
                view! { <HomePage base chapters/> }.into_any()
            }),
        });
        for (i, chapter) in self.chapters.iter().enumerate() {
            pages.push(self.chapter_page(i, chapter));
        }
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
        let body_html = self.markdown().to_html(&chapter.body);
        let numeral = chapter.numeral.clone();
        let title = chapter.title.clone();
        Page {
            path: format!("regras/{}/index.html", chapter.slug),
            html: self.document(&chapter.title, Some(&chapter.summary), move || {
                view! { <ChapterPage numeral title body_html prev next/> }.into_any()
            }),
        }
    }

    /// O documento inteiro de uma página. O corpo é desenhado num contexto de
    /// ilhas: só o que é `#[island]` leva marcas de hidratação.
    fn document(
        &self,
        title: &str,
        description: Option<&str>,
        content: impl FnOnce() -> AnyView + Send + 'static,
    ) -> String {
        let base = self.base.clone();
        let root = base.trim_end_matches('/').to_string();
        let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new_islands())));
        let (scripts, body) = owner.with(move || {
            let options = LeptosOptions::builder()
                .output_name(PKG_NAME)
                .site_pkg_dir("pkg")
                .build();
            let scripts = view! { <HydrationScripts options islands=true root/> }.to_html();
            let body = view! { <Layout base>{content()}</Layout> }.to_html();
            (scripts, body)
        });

        let full_title = if title == "Início" {
            "Centelha".to_string()
        } else {
            format!("{title} · Centelha")
        };
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
<script>{THEME_SCRIPT}</script>\n\
<link rel=\"stylesheet\" href=\"{b}assets/site.css\">\n\
{scripts}\n\
</head>\n\
<body>\n{body}\n</body>\n\
</html>\n",
            title = escape_html(&full_title),
            description = escape_html(description),
        )
    }

    /// Grava as páginas, a folha de estilo e os ícones em `out_dir`. Devolve
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
        self.copy_public_files(out_dir)?;
        Ok(pages)
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
