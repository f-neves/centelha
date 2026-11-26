#!/bin/sh
# Instala as ferramentas de fora do Cargo que o `cargo xtask site` usa
# (ADR 0003): wasm-bindgen-cli e o wasm-opt do binaryen, nas versões fixas,
# baixadas das páginas de release oficiais, com o SHA-256 conferido.
#
# Uso: tools/ci/ferramentas.sh <pasta>
# Grava <pasta>/bin/wasm-bindgen e <pasta>/bin/wasm-opt. Se a pasta já tiver
# as duas (cache do CI), não baixa nada. Só Linux x86_64 (o executor do CI).
#
# Versões e somas conferidas nas páginas de release em 26/11/2026. Subir uma
# versão é trocar as quatro linhas abaixo junto com o ADR 0003 e o xtask.
set -eu

WASM_BINDGEN_VERSAO=0.2.130
WASM_BINDGEN_SHA256=9687f3bb551b9f66f37ed32a40d7913a00f7962db653054e56857bc306c3227f
BINARYEN_VERSAO=version_133
BINARYEN_SHA256=2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489

destino=${1:?uso: ferramentas.sh <pasta>}
mkdir -p "$destino/bin"
destino=$(cd "$destino" && pwd)

baixar() { # url soma arquivo
	curl -fsSL --retry 3 -o "$3" "$1"
	echo "$2  $3" | sha256sum -c -
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

if [ ! -x "$destino/bin/wasm-bindgen" ]; then
	nome=wasm-bindgen-$WASM_BINDGEN_VERSAO-x86_64-unknown-linux-musl
	baixar "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$WASM_BINDGEN_VERSAO/$nome.tar.gz" \
		"$WASM_BINDGEN_SHA256" "$tmp/wb.tar.gz"
	tar -xzf "$tmp/wb.tar.gz" -C "$tmp"
	cp "$tmp/$nome/wasm-bindgen" "$destino/bin/wasm-bindgen"
fi

if [ ! -x "$destino/bin/wasm-opt" ]; then
	nome=binaryen-$BINARYEN_VERSAO
	baixar "https://github.com/WebAssembly/binaryen/releases/download/$BINARYEN_VERSAO/$nome-x86_64-linux.tar.gz" \
		"$BINARYEN_SHA256" "$tmp/bn.tar.gz"
	tar -xzf "$tmp/bn.tar.gz" -C "$tmp" "$nome/bin/wasm-opt"
	cp "$tmp/$nome/bin/wasm-opt" "$destino/bin/wasm-opt"
fi

"$destino/bin/wasm-bindgen" --version
"$destino/bin/wasm-opt" --version
