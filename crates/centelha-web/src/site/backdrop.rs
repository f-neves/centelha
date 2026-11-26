//! O fundo decorativo: o selo da faísca respirando atrás do texto, brasas que
//! sobem, vinheta e grão.
//!
//! Port de `src/components/FundoCentelha.astro` do site atual, sem
//! JavaScript. Lá um laço de `requestAnimationFrame` recalcula a cada quadro
//! os seis losangos, os dois anéis, o brilho e o clarão; aqui o gerador faz a
//! mesma conta em pontos do ciclo e grava uma animação SMIL, que o navegador
//! toca sozinho (como a faísca da marca). As brasas, a vinheta e o grão são CSS
//! puro, copiados de lá. Para quem pede menos movimento vai, ao lado, o mesmo
//! selo parado no quadro que o site atual congela, e o CSS mostra um ou outro.
//!
//! O centro do selo e das camadas segue a coluna do conteúdo
//! (`--fundo-cx`): o site atual mede por script; aqui o CSS calcula pela
//! largura da barra lateral.

use std::f64::consts::PI;
use std::fmt::Write;
use std::sync::LazyLock;

const CX: f64 = 512.0;
const CY: f64 = 512.0;
/// Os parâmetros padrão do `FundoCentelha` do site atual.
const RT: f64 = 450.0;
const GAP: (f64, f64) = (50.0, 100.0);
const NARROW: (f64, f64) = (1.0, 0.7);
const COLOR: (f64, f64) = (0.0, 0.5);
const AURA: f64 = 0.75;
const PULSE: f64 = 0.5;
const SECONDS: f64 = 5.0;
const INTENSITY: f64 = 0.22;
const GOLD: [f64; 3] = [217.0, 184.0, 95.0];
const PALE: [f64; 3] = [248.0, 236.0, 192.0];
/// Pontos do ciclo onde a conta é feita (o navegador interpola entre eles).
const STEPS: usize = 12;

/// O HTML do fundo, igual em todas as páginas.
pub static BACKDROP_HTML: LazyLock<String> = LazyLock::new(build);

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// A intensidade do pulso numa fase do ciclo (0 no vale, 1 no pico).
fn pulse(phase: f64) -> f64 {
    ((1.0 - (phase * 2.0 * PI).cos()) / 2.0).powf(1.4)
}

/// Os seis losangos, como um caminho (as mesmas contas de `markPoints`).
fn mark_path(gap: f64, narrow: f64) -> String {
    let r0 = RT + gap;
    let t = gap / 2.0;
    let hs = 0.288_675_1 * r0;
    let sx = (hs - 1.154_700_5 * t) * narrow;
    let base = [
        (0.0, -2.0 * t),
        (sx, -r0 / 2.0),
        (0.0, -(r0 - 2.0 * t)),
        (-sx, -r0 / 2.0),
    ];
    let mut d = String::new();
    for k in 0..6 {
        let a = k as f64 * PI / 3.0;
        let (ca, sa) = (a.cos(), a.sin());
        for (i, (x, y)) in base.iter().enumerate() {
            let px = CX + x * ca - y * sa;
            let py = CY + x * sa + y * ca;
            let _ = write!(d, "{}{px:.1} {py:.1} ", if i == 0 { "M" } else { "L" });
        }
        d.push_str("Z ");
    }
    d.trim_end().to_string()
}

fn color(m: f64) -> String {
    let c: Vec<String> = GOLD
        .iter()
        .zip(PALE)
        .map(|(a, b)| format!("{}", (a + (b - a) * m).round() as i64))
        .collect();
    format!("rgb({})", c.join(","))
}

/// Um quadro do selo na intensidade `e`.
struct Frame {
    path: String,
    scale: f64,
    fill: String,
    glow_opacity: f64,
    blur: f64,
    flare: f64,
}

fn frame(e: f64) -> Frame {
    Frame {
        path: mark_path(lerp(GAP.0, GAP.1, e), lerp(NARROW.0, NARROW.1, e)),
        scale: 0.96 + 0.05 * e,
        fill: color(lerp(COLOR.0, COLOR.1, e).min(1.0)),
        glow_opacity: AURA * (0.22 + 0.78 * e) * 0.9,
        blur: 4.0 + AURA * 46.0 * (0.35 + 0.65 * e),
        flare: PULSE * (0.1 + 0.9 * e) * 0.6,
    }
}

/// Os anéis numa fase (o mesmo `renderRings`): raio, opacidade e traço.
fn ring(phase: f64) -> (f64, f64, f64) {
    (
        150.0 + phase * 440.0,
        (1.0 - phase).max(0.0) * PULSE * 0.4,
        1.0 + 7.0 * (1.0 - phase),
    )
}

fn join<F: Fn(&Frame) -> String>(frames: &[Frame], f: F) -> String {
    frames.iter().map(f).collect::<Vec<_>>().join(";")
}

/// Uma `<animate>` de um atributo, no ciclo inteiro.
fn animate(attribute: &str, values: &str, times: &str) -> String {
    format!(
        "<animate attributeName=\"{attribute}\" values=\"{values}\" keyTimes=\"{times}\" dur=\"{SECONDS}s\" repeatCount=\"indefinite\"/>"
    )
}

/// O selo: animado (`moving`) ou parado no pico, com os anéis na fase 0,3
/// (o quadro que o site atual mostra para quem pede menos movimento).
fn seal(moving: bool) -> String {
    let still = frame(1.0);
    let (frames, times) = if moving {
        let frames: Vec<Frame> = (0..=STEPS)
            .map(|k| frame(pulse(k as f64 / STEPS as f64)))
            .collect();
        let times = (0..=STEPS)
            .map(|k| format!("{:.4}", k as f64 / STEPS as f64))
            .collect::<Vec<_>>()
            .join(";");
        (frames, times)
    } else {
        (Vec::new(), String::new())
    };
    let id = if moving { "cfa" } else { "cfs" };
    let class = if moving {
        "bg-seal animated"
    } else {
        "bg-seal still"
    };
    let mut svg = format!(
        "<svg class=\"{class}\" viewBox=\"0 0 1024 1024\" preserveAspectRatio=\"xMidYMid meet\" style=\"--seal-intensity:{INTENSITY}\">"
    );
    // O brilho, o clarão e o caminho comum às duas camadas.
    let _ = write!(
        svg,
        "<defs><filter id=\"{id}Glow\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\"><feGaussianBlur stdDeviation=\"{:.1}\">{}</feGaussianBlur></filter>\
         <radialGradient id=\"{id}Flare\" cx=\"50%\" cy=\"50%\" r=\"50%\"><stop offset=\"0%\" stop-color=\"#d9b85f\" stop-opacity=\"0.85\"/><stop offset=\"52%\" stop-color=\"#d9b85f\" stop-opacity=\"0.16\"/><stop offset=\"100%\" stop-color=\"#d9b85f\" stop-opacity=\"0\"/></radialGradient>\
         <path id=\"{id}Shape\" d=\"{}\">{}</path></defs>",
        still.blur,
        if moving {
            animate(
                "stdDeviation",
                &join(&frames, |f| format!("{:.1}", f.blur)),
                &times,
            )
        } else {
            String::new()
        },
        still.path,
        if moving {
            animate("d", &join(&frames, |f| f.path.clone()), &times)
        } else {
            String::new()
        },
    );
    let _ = write!(
        svg,
        "<rect x=\"0\" y=\"0\" width=\"1024\" height=\"1024\" fill=\"url(#{id}Flare)\" opacity=\"{:.3}\">{}</rect>",
        still.flare,
        if moving {
            animate(
                "opacity",
                &join(&frames, |f| format!("{:.3}", f.flare)),
                &times,
            )
        } else {
            String::new()
        },
    );
    svg.push_str(
        "<g stroke=\"currentColor\" fill=\"none\" opacity=\".55\"><circle cx=\"512\" cy=\"512\" r=\"470\" stroke-width=\"2\"/><circle cx=\"512\" cy=\"512\" r=\"410\" stroke-width=\"1.2\"/>\
         <g stroke-width=\"2\"><path d=\"M512 30 v46 M512 948 v46 M30 512 h46 M948 512 h46\"/><path d=\"M150 150 l30 30 M874 150 l-30 30 M150 874 l30 -30 M874 874 l-30 -30\"/></g></g>",
    );
    // Os dois anéis que se abrem, meio ciclo um atrás do outro.
    svg.push_str("<g fill=\"none\">");
    for i in 0..2 {
        if moving {
            let (r0, o0, w0) = ring(0.0);
            let (r1, o1, w1) = ring(1.0);
            let begin = -(i as f64) * SECONDS / 2.0;
            let anim = |attr: &str, a: f64, b: f64| {
                format!(
                    "<animate attributeName=\"{attr}\" values=\"{a:.1};{b:.1}\" dur=\"{SECONDS}s\" begin=\"{begin}s\" repeatCount=\"indefinite\"/>"
                )
            };
            let anim_o = format!(
                "<animate attributeName=\"opacity\" values=\"{o0:.3};{o1:.3}\" dur=\"{SECONDS}s\" begin=\"{begin}s\" repeatCount=\"indefinite\"/>"
            );
            let _ = write!(
                svg,
                "<circle cx=\"512\" cy=\"512\" stroke=\"#e9cf86\" r=\"{r0:.1}\" opacity=\"{o0:.3}\" stroke-width=\"{w0:.2}\">{}{}{}</circle>",
                anim("r", r0, r1),
                anim_o,
                anim("stroke-width", w0, w1),
            );
        } else {
            let (r, o, w) = ring((0.3 + i as f64 * 0.5) % 1.0);
            let _ = write!(
                svg,
                "<circle cx=\"512\" cy=\"512\" stroke=\"#e9cf86\" r=\"{r:.1}\" opacity=\"{o:.3}\" stroke-width=\"{w:.2}\"/>"
            );
        }
    }
    svg.push_str("</g>");
    // O brilho (sem a escala) e o selo (com a escala), sobre o mesmo caminho.
    let _ = write!(
        svg,
        "<g filter=\"url(#{id}Glow)\" fill=\"#d9b85f\" opacity=\"{:.3}\">{}<use href=\"#{id}Shape\"/></g>",
        still.glow_opacity,
        if moving {
            animate(
                "opacity",
                &join(&frames, |f| format!("{:.3}", f.glow_opacity)),
                &times,
            )
        } else {
            String::new()
        },
    );
    let scale_values = join(&frames, |f| format!("{:.4}", f.scale));
    let _ = write!(
        svg,
        "<g transform=\"translate(512 512)\"><g transform=\"scale({:.4})\">{}<g transform=\"translate(-512 -512)\" fill=\"{}\">{}<use href=\"#{id}Shape\"/></g></g></g>",
        still.scale,
        if moving {
            format!(
                "<animateTransform attributeName=\"transform\" type=\"scale\" values=\"{scale_values}\" keyTimes=\"{times}\" dur=\"{SECONDS}s\" repeatCount=\"indefinite\"/>"
            )
        } else {
            String::new()
        },
        still.fill,
        if moving {
            animate("fill", &join(&frames, |f| f.fill.clone()), &times)
        } else {
            String::new()
        },
    );
    svg.push_str("</svg>");
    svg
}

fn build() -> String {
    format!(
        "<div class=\"bg\"></div><div class=\"bg-glow\"></div><div class=\"bg-sparks\"></div><div class=\"bg-sparks2\"></div><div class=\"bg-grain\"></div>{}{}",
        seal(true),
        seal(false)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_valley_and_the_peak_are_the_spark_shapes() {
        // No vale e no pico o selo é a faísca da marca (`CentelhaSpark`).
        assert!(mark_path(50.0, 1.0).starts_with("M512.0 462.0 L627.5 262.0 L512.0 62.0"));
        assert!(mark_path(100.0, 0.7).starts_with("M512.0 412.0 L582.7 237.0 L512.0 62.0"));
        assert_eq!(pulse(0.0), 0.0);
        assert!((pulse(0.5) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn has_a_moving_and_a_still_seal() {
        let html = &*BACKDROP_HTML;
        assert_eq!(html.matches("<svg class=\"bg-seal").count(), 2);
        assert!(html.contains("repeatCount=\"indefinite\""));
        assert!(!html.contains("<script"));
    }
}
