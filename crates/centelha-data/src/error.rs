//! O erro de leitura dos dados.

use std::fmt;

/// O que deu errado ao ler um dado: em qual arquivo, em que ponto do JSON e
/// por quê.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    /// O arquivo ou a pasta não se leu (só no nativo).
    Io { file: String, reason: String },
    /// O texto não é JSON, ou o JSON não tem a forma do tipo.
    Json {
        file: Option<String>,
        path: String,
        reason: String,
    },
    /// O JSON tem a forma do tipo, mas fere uma regra da validação dos dados.
    Invalid {
        file: Option<String>,
        path: String,
        reason: String,
    },
}

impl DataError {
    /// O arquivo do erro, quando se sabe.
    pub fn file(&self) -> Option<&str> {
        match self {
            DataError::Io { file, .. } => Some(file),
            DataError::Json { file, .. } | DataError::Invalid { file, .. } => file.as_deref(),
        }
    }

    /// O caminho dentro do JSON (`ataques[2].pgr`), vazio quando o erro é do
    /// arquivo inteiro.
    pub fn path(&self) -> &str {
        match self {
            DataError::Io { .. } => "",
            DataError::Json { path, .. } | DataError::Invalid { path, .. } => path,
        }
    }

    /// O motivo, em texto.
    pub fn reason(&self) -> &str {
        match self {
            DataError::Io { reason, .. }
            | DataError::Json { reason, .. }
            | DataError::Invalid { reason, .. } => reason,
        }
    }

    /// O mesmo erro, dizendo de qual arquivo veio.
    pub fn in_file(self, name: impl Into<String>) -> Self {
        let name = Some(name.into());
        match self {
            DataError::Json { path, reason, .. } => DataError::Json {
                file: name,
                path,
                reason,
            },
            DataError::Invalid { path, reason, .. } => DataError::Invalid {
                file: name,
                path,
                reason,
            },
            io @ DataError::Io { .. } => io,
        }
    }

    pub(crate) fn invalid(path: impl Into<String>, reason: impl Into<String>) -> Self {
        DataError::Invalid {
            file: None,
            path: path.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self.file().unwrap_or("(texto)");
        match self.path() {
            "" => write!(f, "{file}: {}", self.reason()),
            path => write!(f, "{file}, em {path}: {}", self.reason()),
        }
    }
}

impl std::error::Error for DataError {}
