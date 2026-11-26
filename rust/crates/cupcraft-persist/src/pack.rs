//! Contenedor `.cupcraft`: un archivo de proyecto empaquetado (ZIP real,
//! método *store* sin compresión) que reúne en un solo fichero portable:
//!
//! ```text
//! project.json          → el ProjectFile (proyecto + metadatos de medios)
//! assets/<id>.<ext>      → los bytes crudos de cada medio
//! ```
//!
//! Al compartir el `.cupcraft` con otra persona, esta lo abre y sigue editando
//! sin perder referencias: los clips referencian los medios por `media_id`, y
//! al importar se recrean los registros de medios desde `assets/`.
//!
//! Se escribe como ZIP estándar (método 0 = *store*, con CRC-32 por entrada),
//! así cualquier descompresor lo abre; y se lee por el directorio central, el
//! camino canónico del formato. Sin dependencias externas: solo `std`.

/// Extensión del contenedor nativo de cupcraft.
pub const CUPCRAFT_EXT: &str = "cupcraft";

/// Firma de un archivo ZIP (`PK\x03\x04`): sirve para distinguir un `.cupcraft`
/// de un `.opencup.json` de texto al abrir.
pub const ZIP_MAGIC: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];

/// Error al leer un contenedor `.cupcraft`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    /// No es un ZIP válido o está truncado.
    Invalid,
    /// Usa compresión (cupcraft solo escribe método *store*).
    Unsupported,
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::Invalid => f.write_str("el contenedor .cupcraft no es válido o está incompleto"),
            PackError::Unsupported => f.write_str("el contenedor usa un formato de compresión no soportado"),
        }
    }
}

impl std::error::Error for PackError {}

/// Extensión de archivo para un tipo MIME (igual criterio que el almacén).
pub fn ext_for_mime(mime: &str) -> &'static str {
    match mime {
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "audio/wav" => "wav",
        "audio/mp3" | "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        _ => "bin",
    }
}

/// CRC-32 (polinomio IEEE 0xEDB88320), el que exige el formato ZIP por entrada.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Escribe un ZIP (método *store*) con las entradas `(ruta, bytes)` dadas.
pub fn write_zip(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    // (offset de la cabecera local, crc, tamaño, nombre) para el directorio central.
    let mut index: Vec<(u32, u32, u32, Vec<u8>)> = Vec::new();

    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        let name_b = name.as_bytes();
        // Cabecera local de archivo.
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // versión necesaria
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // método (0 = store)
        out.extend_from_slice(&0u16.to_le_bytes()); // hora
        out.extend_from_slice(&0x0021u16.to_le_bytes()); // fecha (1980-01-01)
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes()); // tamaño comprimido
        out.extend_from_slice(&size.to_le_bytes()); // tamaño sin comprimir
        out.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // longitud extra
        out.extend_from_slice(name_b);
        out.extend_from_slice(data);
        index.push((offset, crc, size, name_b.to_vec()));
    }

    let cd_offset = out.len() as u32;
    for (offset, crc, size, name_b) in &index {
        out.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // versión creador
        out.extend_from_slice(&20u16.to_le_bytes()); // versión necesaria
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // método
        out.extend_from_slice(&0u16.to_le_bytes()); // hora
        out.extend_from_slice(&0x0021u16.to_le_bytes()); // fecha
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(&0u16.to_le_bytes()); // comentario
        out.extend_from_slice(&0u16.to_le_bytes()); // disco inicial
        out.extend_from_slice(&0u16.to_le_bytes()); // atributos internos
        out.extend_from_slice(&0u32.to_le_bytes()); // atributos externos
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(name_b);
    }
    let cd_size = (out.len() as u32) - cd_offset;

    // Fin del directorio central (EOCD).
    let count = index.len() as u16;
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // número de disco
    out.extend_from_slice(&0u16.to_le_bytes()); // disco del directorio central
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // longitud del comentario
    out
}

#[inline]
fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(at)?, *d.get(at + 1)?]))
}

#[inline]
fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([*d.get(at)?, *d.get(at + 1)?, *d.get(at + 2)?, *d.get(at + 3)?]))
}

/// Lee un ZIP (método *store*) por su directorio central y devuelve las
/// entradas `(ruta, bytes)`.
pub fn read_zip(data: &[u8]) -> Result<Vec<(String, Vec<u8>)>, PackError> {
    if data.len() < 22 {
        return Err(PackError::Invalid);
    }
    // Localiza el EOCD buscando su firma desde el final (admite comentario).
    let eocd_sig = 0x0605_4b50u32.to_le_bytes();
    let lower = data.len().saturating_sub(22 + 0xFFFF);
    let mut eocd = None;
    for i in (lower..=data.len() - 22).rev() {
        if data[i..i + 4] == eocd_sig {
            eocd = Some(i);
            break;
        }
    }
    let e = eocd.ok_or(PackError::Invalid)?;
    let count = u16_at(data, e + 10).ok_or(PackError::Invalid)? as usize;
    let mut p = u32_at(data, e + 16).ok_or(PackError::Invalid)? as usize;

    let cd_sig = 0x0201_4b50u32.to_le_bytes();
    let lh_sig = 0x0403_4b50u32.to_le_bytes();
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if data.get(p..p + 4) != Some(&cd_sig[..]) {
            return Err(PackError::Invalid);
        }
        let method = u16_at(data, p + 10).ok_or(PackError::Invalid)?;
        let comp_size = u32_at(data, p + 20).ok_or(PackError::Invalid)? as usize;
        let name_len = u16_at(data, p + 28).ok_or(PackError::Invalid)? as usize;
        let extra_len = u16_at(data, p + 30).ok_or(PackError::Invalid)? as usize;
        let comment_len = u16_at(data, p + 32).ok_or(PackError::Invalid)? as usize;
        let local = u32_at(data, p + 42).ok_or(PackError::Invalid)? as usize;
        if method != 0 {
            return Err(PackError::Unsupported);
        }
        let name = data.get(p + 46..p + 46 + name_len).ok_or(PackError::Invalid)?;
        let name = String::from_utf8_lossy(name).into_owned();
        // Cabecera local: el nombre/extra pueden diferir en longitud, así que se releen.
        if data.get(local..local + 4) != Some(&lh_sig[..]) {
            return Err(PackError::Invalid);
        }
        let l_name = u16_at(data, local + 26).ok_or(PackError::Invalid)? as usize;
        let l_extra = u16_at(data, local + 28).ok_or(PackError::Invalid)? as usize;
        let start = local + 30 + l_name + l_extra;
        let end = start.checked_add(comp_size).ok_or(PackError::Invalid)?;
        let bytes = data.get(start..end).ok_or(PackError::Invalid)?.to_vec();
        out.push((name, bytes));
        p += 46 + name_len + extra_len + comment_len;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_round_trip_preserves_entries() {
        let entries = vec![
            ("project.json".to_string(), br#"{"kind":"openCUP-project"}"#.to_vec()),
            ("assets/m1.png".to_string(), vec![1, 2, 3, 4, 5, 6, 7, 8]),
            ("assets/m2.mp4".to_string(), (0u8..200).collect()),
        ];
        let zip = write_zip(&entries);
        assert_eq!(&zip[0..4], &ZIP_MAGIC, "empieza por la firma PK\\x03\\x04");
        let read = read_zip(&zip).unwrap();
        assert_eq!(read, entries);
    }

    #[test]
    fn empty_container_is_valid() {
        let zip = write_zip(&[]);
        assert_eq!(read_zip(&zip).unwrap(), Vec::new());
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(read_zip(b"no soy un zip").unwrap_err(), PackError::Invalid);
        assert_eq!(read_zip(&[]).unwrap_err(), PackError::Invalid);
    }

    #[test]
    fn crc32_matches_known_vector() {
        // CRC-32 de "123456789" = 0xCBF43926 (vector de referencia).
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
