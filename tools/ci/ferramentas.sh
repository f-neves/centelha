#!/bin/sh
# Instala as ferramentas de fora do Cargo que o `cargo xtask site` usa
# (ADR 0003): wasm-bindgen-cli, o wasm-opt do binaryen e o Pagefind, nas versões fixas,
# baixadas das páginas de release oficiais, com o SHA-256 conferido.
#
# Uso: tools/ci/ferramentas.sh <pasta>
# Grava <pasta>/bin/wasm-bindgen, wasm-opt e pagefind. Se a pasta já tiver
# as três (cache do CI), não baixa nada. Só Linux x86_64 (o executor do CI).
#
# Versões e somas conferidas nas páginas de release em 26/11/2026. Subir uma
# versão é trocar as linhas abaixo junto com o ADR 0003 e o xtask.
set -eu

WASM_BINDGEN_VERSAO=0.2.130
WASM_BINDGEN_SHA256=9687f3bb551b9f66f37ed32a40d7913a00f7962db653054e56857bc306c3227f
BINARYEN_VERSAO=version_133
BINARYEN_SHA256=2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489
# Pagefind (índice da busca), conferido em 26/11/2026.
PAGEFIND_VERSAO=1.6.0
PAGEFIND_SHA256=004d40c9a644c995dadc42b8031196c6f17e7a6dacf9a6c15c63286dd6ead5f2

destino=${1:?uso: ferramentas.sh <pasta>}
mkdir -p "$destino/bin"
destino=$(cd "$destino" && pwd)

baixar() { # ferramenta url soma arquivo
	curl -fsSL --retry 3 -o "$4" "$2"
	obtida=$(sha256sum "$4" | cut -d' ' -f1)
	if [ "$obtida" != "$3" ]; then
		echo "ferramentas.sh: SHA-256 errado no $1" >&2
		echo "  url:      $2" >&2
		echo "  esperado: $3" >&2
		echo "  obtido:   $obtida" >&2
		exit 1
	fi
}

# Sai 0 se o binário existe e diz exatamente a versão pedida. Um cache velho
# (outra versão no mesmo lugar) não passa: é baixado de novo.
versao_certa() { # binário texto-esperado
	[ -x "$1" ] && [ "$("$1" --version 2>/dev/null)" = "$2" ]
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

WB_TEXTO="wasm-bindgen $WASM_BINDGEN_VERSAO"
if ! versao_certa "$destino/bin/wasm-bindgen" "$WB_TEXTO"; then
	nome=wasm-bindgen-$WASM_BINDGEN_VERSAO-x86_64-unknown-linux-musl
	baixar wasm-bindgen "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$WASM_BINDGEN_VERSAO/$nome.tar.gz" \
		"$WASM_BINDGEN_SHA256" "$tmp/wb.tar.gz"
	tar -xzf "$tmp/wb.tar.gz" -C "$tmp"
	cp "$tmp/$nome/wasm-bindgen" "$destino/bin/wasm-bindgen"
fi

BN_TEXTO="wasm-opt version ${BINARYEN_VERSAO#version_} ($BINARYEN_VERSAO)"
if ! versao_certa "$destino/bin/wasm-opt" "$BN_TEXTO"; then
	nome=binaryen-$BINARYEN_VERSAO
	baixar binaryen "https://github.com/WebAssembly/binaryen/releases/download/$BINARYEN_VERSAO/$nome-x86_64-linux.tar.gz" \
		"$BINARYEN_SHA256" "$tmp/bn.tar.gz"
	tar -xzf "$tmp/bn.tar.gz" -C "$tmp" "$nome/bin/wasm-opt"
	cp "$tmp/$nome/bin/wasm-opt" "$destino/bin/wasm-opt"
fi

PF_TEXTO="pagefind $PAGEFIND_VERSAO"
if ! versao_certa "$destino/bin/pagefind" "$PF_TEXTO"; then
	nome=pagefind-v$PAGEFIND_VERSAO-x86_64-unknown-linux-musl
	baixar pagefind "https://github.com/Pagefind/pagefind/releases/download/v$PAGEFIND_VERSAO/$nome.tar.gz" 		"$PAGEFIND_SHA256" "$tmp/pf.tar.gz"
	tar -xzf "$tmp/pf.tar.gz" -C "$tmp" pagefind
	cp "$tmp/pagefind" "$destino/bin/pagefind"
fi

# Confere de novo depois de baixar: versão errada aqui para o passo.
for par in "wasm-bindgen|$WB_TEXTO" "wasm-opt|$BN_TEXTO" "pagefind|$PF_TEXTO"; do
	bin=${par%%|*}; texto=${par#*|}
	if ! versao_certa "$destino/bin/$bin" "$texto"; then
		echo "ferramentas.sh: $bin diz \"$("$destino/bin/$bin" --version 2>&1)\", esperado \"$texto\"" >&2
		exit 1
	fi
done
"$destino/bin/wasm-bindgen" --version
"$destino/bin/wasm-opt" --version
"$destino/bin/pagefind" --version
