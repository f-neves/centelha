# Termos proibidos (R9, R11), lidos pelos ganchos commit-msg e pre-commit.
# Montados por partes para que este arquivo não seja ele mesmo um achado.
# A mesma lista mora em xtask/src/scan.rs (varredura do CI): mude as duas.

# Trecho em qualquer posição, sem diferença de caixa.
TERMOS_TRECHO="cla""ude|anthro""pic|co""dex|open""ai|cha""tgpt|copi""lot|gem""ini|co-authored""-by|generated ""with|c2""pa|trained""algorithmic""media"
# Palavra curta: sem letra colada antes ou depois.
# Inclui os nomes internos de papel trocados nas notas (ADR 0005).
TERMOS_PALAVRA="(^|[^a-z])(g""pt|l""lm|arqui""tetos?|arqui""teta|bestia""ristas?)([^a-z]|$)"
# Marcadores dentro de imagem e de arquivo binário.
TERMOS_BINARIO="cla""ude|anthro""pic|open""ai|cha""tgpt|g""pt-image|c2""pa|trained""algorithmic""media"

# Imprime os termos achados na entrada padrão (texto).
achados_texto() {
	grep -o -i -E "$TERMOS_TRECHO" | sort -u
	true
}
achados_texto_e_palavra() {
	entrada=$(cat)
	printf '%s' "$entrada" | grep -o -i -E "$TERMOS_TRECHO" | sort -u
	printf '%s' "$entrada" | grep -o -i -E "$TERMOS_PALAVRA" | sort -u
	true
}
