//! A Força na ficha: o alcance do arremesso (port de `alcanceArremesso`,
//! `src/lib/forca-empurrao.ts`).

use centelha_data::Rules;

use crate::js;

/// `alcanceArremesso`: até onde vai um objeto de `weight_kg`, pela Força
/// aplicada (`reserve`, o FAA) e pelo peso máximo erguido (`max_kg`). O
/// objeto acima do teto do arremesso (uma fração do peso máximo) não voa;
/// abaixo do ápice a velocidade do braço satura; perto do teto o alcance cai
/// até zero.
pub fn throw_range(rules: &Rules, reserve: f64, weight_kg: f64, max_kg: f64) -> f64 {
    let f = &rules.strength.throw;
    let get = |x: Option<f64>| x.unwrap_or(f64::NAN);
    let cap_kg = max_kg * get(f.cap_fraction);
    if weight_kg <= 0.0 || weight_kg > cap_kg {
        return 0.0;
    }
    let apex = get(f.apex_kg);
    let head = get(f.constant) * js::pow(reserve, get(f.reserve_exponent));
    let heavy = |w: f64| head / js::pow(w, get(f.mass_exponent));
    let light = |w: f64| heavy(apex) * js::pow(w / apex, get(f.light_exponent));
    let drop_from = cap_kg * get(f.drop_fraction);
    let d = if weight_kg < apex {
        light(weight_kg)
    } else {
        heavy(weight_kg)
    };
    if weight_kg > drop_from {
        d * (cap_kg - weight_kg) / (cap_kg - drop_from)
    } else {
        d
    }
}
