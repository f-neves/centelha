//! A navegação do site: os capítulos da barra lateral, as ferramentas, a
//! trilha de navegação e os endereços.
//!
//! Port de `NAV` e `FERRAMENTAS` (`src/lib/site.ts` do site atual) e da trilha
//! montada em `src/layouts/Base.astro`. A lista é do site, não do conteúdo:
//! os títulos da barra lateral não são os títulos das páginas (o capítulo II
//! se chama "Atributos & Habilidades" na barra, e a página "Habilidades
//! Secundárias" aparece como "Secundárias"). O teste confere que cada capítulo
//! de `content/chapters` está aqui.

/// Uma página de um capítulo de várias páginas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubItem {
    pub slug: &'static str,
    pub title: &'static str,
}

/// Uma linha da barra lateral (o primeiro item é a página inicial, sem
/// numeral).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavItem {
    pub slug: &'static str,
    pub title: &'static str,
    pub numeral: &'static str,
    /// As páginas do capítulo, que só aparecem enquanto se está nele.
    pub sub: &'static [SubItem],
}

const fn item(slug: &'static str, title: &'static str, numeral: &'static str) -> NavItem {
    NavItem {
        slug,
        title,
        numeral,
        sub: &[],
    }
}

const fn sub(slug: &'static str, title: &'static str) -> SubItem {
    SubItem { slug, title }
}

/// A barra lateral, na ordem do site atual.
pub const NAV: &[NavItem] = &[
    item("", "Início", ""),
    item("regras/coracao-do-sistema", "O Coração do Sistema", "I"),
    NavItem {
        slug: "regras/atributos",
        title: "Atributos & Habilidades",
        numeral: "II",
        sub: &[
            sub("regras/atributos", "Atributos"),
            sub("regras/habilidades", "Habilidades"),
            sub("regras/habilidades-secundarias", "Secundárias"),
        ],
    },
    item(
        "regras/aparencia-virtudes-vontade",
        "Aparência, Virtudes & Força de Vontade",
        "III",
    ),
    item(
        "regras/vida-ferimentos-cura",
        "Vida, Ferimentos & Cura",
        "IV",
    ),
    item("regras/centelha", "Centelha", "V"),
    item("regras/racas", "Raças", "VI"),
    item("regras/antecedentes", "Antecedentes", "VII"),
    NavItem {
        slug: "regras/acoes-e-sistema",
        title: "Ações & Sistema",
        numeral: "VIII",
        sub: &[
            sub("regras/acoes-e-sistema", "A Régua Comum"),
            sub("regras/acoes-corpo-e-movimento", "Corpo e Movimento"),
            sub("regras/acoes-resistir", "Resistir"),
            sub("regras/acoes-sentidos-e-engano", "Sentidos & Engano"),
            sub("regras/acoes-oficio-e-mundo", "Ofício e Mundo"),
        ],
    },
    NavItem {
        slug: "regras/combate",
        title: "Combate Físico",
        numeral: "IX",
        sub: &[
            sub("regras/combate", "Combate Físico"),
            sub("regras/condicoes", "Condições"),
            sub("regras/poderes-das-criaturas", "Os Poderes das Criaturas"),
        ],
    },
    item("regras/relacoes-sociais", "Relações Sociais", "X"),
    item("regras/defesas", "As Três Defesas", "XI"),
    item("regras/quase-acerto", "Quase-Acerto", "XII"),
    item("regras/armas-e-armaduras", "Armas & Armaduras", "XIII"),
    NavItem {
        slug: "regras/custo-de-servico-e-itens",
        title: "Custo de Serviço & Itens",
        numeral: "XIV",
        sub: &[
            sub(
                "regras/custo-de-servico-e-itens",
                "Moeda, Renda e Custo de Vida",
            ),
            sub("regras/custo-servicos", "Serviços e Contratação"),
            sub("regras/custo-mercadorias", "Mercadorias"),
            sub(
                "regras/custo-qualidade-e-equipamento",
                "Qualidade e Equipamento",
            ),
            sub(
                "regras/custo-montarias-e-viagens",
                "Montarias, Veículos e Viagens",
            ),
        ],
    },
    item("caminhos", "As Proezas", "XV"),
    item("arcano", "O Arcano", "XVI"),
    NavItem {
        slug: "artes/regras",
        title: "As Artes",
        numeral: "XVII",
        sub: &[
            sub("artes/regras", "Regras"),
            sub("artes/efeitos", "Efeitos Especiais"),
            sub("artes/catalogo", "As 24 Artes"),
        ],
    },
    item(
        "regras/criacao-de-personagem",
        "Criação de Personagem",
        "XVIII",
    ),
    item("regras/qual-sistema", "Qual Sistema Eu Uso?", "XIX"),
];

/// Uma ferramenta (grupo "Ferramentas" da barra lateral e da página inicial).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tool {
    pub slug: &'static str,
    pub title: &'static str,
}

/// As ferramentas, na ordem do site atual.
pub const TOOLS: &[Tool] = &[
    Tool {
        slug: "mestre",
        title: "Área do Mestre",
    },
    Tool {
        slug: "ficha",
        title: "Ficha de Personagem",
    },
    Tool {
        slug: "bestiario",
        title: "Bestiário & NPCs",
    },
    Tool {
        slug: "equipamentos",
        title: "Equipamentos",
    },
    Tool {
        slug: "rolador",
        title: "Rolador de Dados",
    },
    Tool {
        slug: "recompensa",
        title: "Calculadora de Recompensa",
    },
    Tool {
        slug: "tecnicas",
        title: "Técnicas (filtros)",
    },
    Tool {
        slug: "arvore",
        title: "Árvores de Técnicas",
    },
    Tool {
        slug: "glossario",
        title: "Glossário",
    },
    Tool {
        slug: "marcadores",
        title: "Meus Marcadores",
    },
];

/// O endereço de uma página do site: o caminho base mais o caminho da página,
/// com `/` no fim (a pasta da página). Um `#âncora` no caminho fica depois da
/// barra (`glossario#tecnica` vira `<base>glossario/#tecnica`).
pub fn url(base: &str, path: &str) -> String {
    let (page, fragment) = match path.split_once('#') {
        Some((page, fragment)) => (page, Some(fragment)),
        None => (path, None),
    };
    let page = page.trim_matches('/');
    let mut out = if page.is_empty() {
        base.to_string()
    } else {
        format!("{base}{page}/")
    };
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

/// O capítulo de várias páginas em que a página `path` está, se estiver.
pub fn chapter_of(path: &str) -> Option<&'static NavItem> {
    NAV.iter().find(|n| n.sub.iter().any(|s| s.slug == path))
}

/// Um passo da trilha de navegação; sem endereço, sai como texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub label: String,
    pub href: Option<String>,
}

/// A trilha de navegação da página `path` (vazio na página inicial), como no
/// site atual: Início, a seção da barra lateral (Regras ou Ferramentas, sem
/// endereço), o capítulo de várias páginas e a página.
pub fn crumbs(base: &str, path: &str, title: &str) -> Vec<Crumb> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut crumbs = vec![Crumb {
        label: "Início".to_string(),
        href: Some(url(base, "")),
    }];
    match parts.first() {
        Some(&"regras") => crumbs.push(Crumb {
            label: "Regras".to_string(),
            href: None,
        }),
        Some(first) if TOOLS.iter().any(|t| t.slug == *first) => crumbs.push(Crumb {
            label: "Ferramentas".to_string(),
            href: None,
        }),
        _ => {}
    }
    if parts.first() == Some(&"caminhos") && parts.len() > 1 {
        crumbs.push(Crumb {
            label: "As Proezas".to_string(),
            href: Some(url(base, "caminhos")),
        });
    }
    if let Some(chapter) = chapter_of(path) {
        crumbs.push(Crumb {
            label: chapter.title.to_string(),
            href: Some(url(base, chapter.slug)),
        });
    }
    if !parts.is_empty() {
        crumbs.push(Crumb {
            label: title.to_string(),
            href: None,
        });
    }
    crumbs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_end_in_a_slash_and_keep_the_fragment() {
        assert_eq!(url("/centelha/", ""), "/centelha/");
        assert_eq!(url("/", "regras/combate"), "/regras/combate/");
        assert_eq!(
            url("/centelha/", "glossario#tecnica"),
            "/centelha/glossario/#tecnica"
        );
    }

    #[test]
    fn crumbs_like_the_current_site() {
        let labels = |path: &str, title: &str| -> Vec<String> {
            crumbs("/", path, title)
                .into_iter()
                .map(|c| c.label)
                .collect()
        };
        assert_eq!(
            labels("regras/condicoes", "Condições"),
            ["Início", "Regras", "Combate Físico", "Condições"]
        );
        assert_eq!(
            labels("regras/defesas", "As Três Defesas"),
            ["Início", "Regras", "As Três Defesas"]
        );
        assert_eq!(
            labels("glossario", "Glossário"),
            ["Início", "Ferramentas", "Glossário"]
        );
        assert_eq!(labels("", "Início"), ["Início"]);
        let regras = &crumbs("/", "regras/defesas", "x")[1];
        assert_eq!(regras.href, None);
    }
}
