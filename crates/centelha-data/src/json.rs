//! Leitura de JSON com o caminho do erro, comum a todos os arquivos de dados.

use serde::de::DeserializeOwned;

use crate::DataError;

/// Lê um valor do texto do JSON. O erro diz o caminho dentro do JSON.
pub(crate) fn parse<T: DeserializeOwned>(json: &str) -> Result<T, DataError> {
    let mut de = serde_json::Deserializer::from_str(json);
    let value: T = serde_path_to_error::deserialize(&mut de).map_err(|e| DataError::Json {
        file: None,
        path: e.path().to_string(),
        reason: e.inner().to_string(),
    })?;
    de.end().map_err(|e| DataError::Json {
        file: None,
        path: String::new(),
        reason: e.to_string(),
    })?;
    Ok(value)
}

/// Lê um arquivo de `<content_dir>/data/` como texto (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_data_file(
    content_dir: &std::path::Path,
    name: &str,
) -> Result<(String, String), DataError> {
    let path = content_dir.join("data").join(name);
    let text = std::fs::read_to_string(&path).map_err(|e| DataError::Io {
        file: path.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok((format!("data/{name}"), text))
}
