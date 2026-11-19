//! Deserializador del formato V8 SerializedScriptValue utilizado por Chromium / WebView2 en IndexedDB.
//!
//! Convierte los registros binarios serializados por V8 en estructuras `serde_json::Value`
//! para migrar proyectos antiguos a SQLite sin dependencias externas.

use serde_json::{Map, Number, Value};

pub struct V8Reader<'a> {
    data: &'a [u8],
    pos: usize,
    references: Vec<Value>,
}

impl<'a> V8Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            references: Vec::new(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn read_byte(&mut self) -> Option<u8> {
        let b = self.data.get(self.pos).copied()?;
        self.pos += 1;
        Some(b)
    }

    fn read_varint(&mut self) -> Option<u64> {
        let mut result = 0u64;
        let mut shift = 0;
        loop {
            let b = self.read_byte()?;
            result |= ((b & 0x7F) as u64) << shift;
            if (b & 0x80) == 0 {
                break;
            }
            shift += 7;
            if shift >= 64 {
                return None;
            }
        }
        Some(result)
    }

    fn read_f64(&mut self) -> Option<f64> {
        if self.pos + 8 > self.data.len() {
            return None;
        }
        let bytes: [u8; 8] = self.data[self.pos..self.pos + 8].try_into().ok()?;
        self.pos += 8;
        Some(f64::from_le_bytes(bytes))
    }

    pub fn deserialize(&mut self) -> Option<Value> {
        // Omitir cabecera de versión de V8: FF <version>
        if self.peek() == Some(0xFF) {
            self.read_byte();
            self.read_varint();
        }
        // Algunos volcados de IndexedDB anteponen metadatos de payload (FF 15 ... FF 10)
        while self.peek() == Some(0xFF) {
            self.read_byte();
            let tag = self.read_byte()?;
            if tag == 0x10 {
                // Comienza el cuerpo serializado
                break;
            } else {
                let len = self.read_varint()?;
                self.pos += len as usize;
            }
        }
        self.read_value()
    }

    fn read_value(&mut self) -> Option<Value> {
        let tag = self.read_byte()?;
        match tag {
            0x5F => Some(Value::Null), // undefined
            0x30 => Some(Value::Null), // null
            0x54 => Some(Value::Bool(true)), // true
            0x46 => Some(Value::Bool(false)), // false
            0x49 => {
                // Int32 (zigzag)
                let n = self.read_varint()?;
                let signed = ((n >> 1) as i64) ^ (-((n & 1) as i64));
                Some(Value::Number(Number::from(signed)))
            }
            0x55 => {
                // UInt32
                let n = self.read_varint()?;
                Some(Value::Number(Number::from(n)))
            }
            0x4E => {
                // Double
                let f = self.read_f64()?;
                Number::from_f64(f).map(Value::Number)
            }
            0x22 => {
                // OneByteString (Latin1/ASCII)
                let len = self.read_varint()? as usize;
                if self.pos + len > self.data.len() {
                    return None;
                }
                let bytes = &self.data[self.pos..self.pos + len];
                self.pos += len;
                let s = String::from_utf8_lossy(bytes).to_string();
                Some(Value::String(s))
            }
            0x63 => {
                // TwoByteString (UTF-16LE)
                let byte_len = self.read_varint()? as usize;
                if self.pos + byte_len > self.data.len() {
                    return None;
                }
                let slice = &self.data[self.pos..self.pos + byte_len];
                self.pos += byte_len;
                let u16s: Vec<u16> = slice
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                let s = String::from_utf16_lossy(&u16s);
                Some(Value::String(s))
            }
            0x6F => {
                // Objeto JS: pares clave-valor hasta 0x7B '{'
                let mut map = Map::new();
                loop {
                    if self.peek() == Some(0x7B) {
                        self.read_byte();
                        self.read_varint(); // número de propiedades
                        break;
                    }
                    if self.pos >= self.data.len() {
                        break;
                    }
                    let key = match self.read_value()? {
                        Value::String(k) => k,
                        other => other.to_string(),
                    };
                    let val = self.read_value()?;
                    map.insert(key, val);
                }
                Some(Value::Object(map))
            }
            0x61 | 0x41 => {
                // Array denso
                let len = self.read_varint()? as usize;
                let mut arr = Vec::with_capacity(len.min(1000));
                for _ in 0..len {
                    if self.peek() == Some(0x24) {
                        break;
                    }
                    if let Some(val) = self.read_value() {
                        arr.push(val);
                    } else {
                        break;
                    }
                }
                // Omitir propiedades extra y delimitador '$' (0x24)
                while let Some(b) = self.read_byte() {
                    if b == 0x24 {
                        self.read_varint(); // length de cierre
                        break;
                    }
                }
                Some(Value::Array(arr))
            }
            0x72 => {
                // Referencia a objeto anterior
                let id = self.read_varint()? as usize;
                self.references.get(id).cloned()
            }
            _ => {
                // Tag no reconocido o padding
                None
            }
        }
    }
}

/// Parsea un buffer V8 a serde_json::Value.
pub fn parse_v8(data: &[u8]) -> Option<Value> {
    let mut reader = V8Reader::new(data);
    reader.deserialize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v8_string_and_object() {
        // [FF 11 6F 22 02 'i' 'd' 22 03 'p' '0' '1' 7B 01]
        let bytes = [
            0xFF, 0x11, 0x6F, 0x22, 0x02, b'i', b'd', 0x22, 0x03, b'p', b'0', b'1', 0x7B, 0x01,
        ];
        let val = parse_v8(&bytes).expect("parse object");
        assert_eq!(val["id"], "p01");
    }
}
