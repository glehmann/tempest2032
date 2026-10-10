//! Binary KeyValues with opaque scalar payloads and ordered, lossless objects.

use super::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Object {
    pub fields: Vec<Field>,
    end: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Field {
    key: Vec<u8>,
    kind: u8,
    value: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Object(Object),
    Raw(Vec<u8>),
}

impl Object {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            end: 8,
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut input = bytes;
        let object = Self::read(&mut input, 0)?;
        if !input.is_empty() {
            return Err("Trailing data in binary VDF".into());
        }
        Ok(object)
    }

    fn read(input: &mut &[u8], depth: usize) -> Result<Self> {
        if depth > 64 {
            return Err("Binary VDF nesting exceeds 64 levels".into());
        }
        let mut fields = Vec::new();
        loop {
            let kind = take(input, 1)?[0];
            if matches!(kind, 8 | 11) {
                return Ok(Self { fields, end: kind });
            }
            let key = cstring(input)?.strip_suffix(&[0]).unwrap().to_vec();
            let value = match kind {
                0 => Value::Object(Self::read(input, depth + 1)?),
                1 => Value::Raw(cstring(input)?.to_vec()),
                2 | 3 | 4 | 6 => Value::Raw(take(input, 4)?.to_vec()),
                5 => {
                    let length = input
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .position(|p| *p == [0, 0])
                        .ok_or("Unterminated binary VDF wide string")?;
                    Value::Raw(take(input, (length + 1) * 2)?.to_vec())
                }
                7 | 10 => Value::Raw(take(input, 8)?.to_vec()),
                _ => return Err(format!("Unsupported binary VDF type {kind}").into()),
            };
            fields.push(Field { key, kind, value });
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.write(&mut bytes);
        bytes
    }

    fn write(&self, bytes: &mut Vec<u8>) {
        for field in &self.fields {
            bytes.push(field.kind);
            bytes.extend_from_slice(&field.key);
            bytes.push(0);
            match &field.value {
                Value::Object(object) => object.write(bytes),
                Value::Raw(value) => bytes.extend_from_slice(value),
            }
        }
        bytes.push(self.end);
    }

    pub fn get(&self, key: &str) -> Option<&Field> {
        self.fields
            .iter()
            .find(|f| f.key.eq_ignore_ascii_case(key.as_bytes()))
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Field> {
        self.fields
            .iter_mut()
            .find(|f| f.key.eq_ignore_ascii_case(key.as_bytes()))
    }

    pub fn set(&mut self, field: Field) {
        if let Some(existing) = self
            .fields
            .iter_mut()
            .find(|f| f.key.eq_ignore_ascii_case(&field.key))
        {
            existing.kind = field.kind;
            existing.value = field.value;
        } else {
            self.fields.push(field);
        }
    }
}

impl Field {
    pub fn object(key: &str, object: Object) -> Self {
        Self {
            key: key.as_bytes().to_vec(),
            kind: 0,
            value: Value::Object(object),
        }
    }

    pub fn string(key: &str, value: &str) -> Self {
        let mut value = value.as_bytes().to_vec();
        value.push(0);
        Self {
            key: key.as_bytes().to_vec(),
            kind: 1,
            value: Value::Raw(value),
        }
    }

    pub fn integer(key: &str, value: u32) -> Self {
        Self {
            key: key.as_bytes().to_vec(),
            kind: 2,
            value: Value::Raw(value.to_le_bytes().to_vec()),
        }
    }

    pub fn key(&self) -> Option<&str> {
        std::str::from_utf8(&self.key).ok()
    }

    pub fn as_object(&self) -> Option<&Object> {
        if let Value::Object(object) = &self.value {
            Some(object)
        } else {
            None
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Object> {
        if let Value::Object(object) = &mut self.value {
            Some(object)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if self.kind == 1
            && let Value::Raw(value) = &self.value
        {
            std::str::from_utf8(value.strip_suffix(&[0])?).ok()
        } else {
            None
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        if self.kind == 2
            && let Value::Raw(value) = &self.value
        {
            Some(u32::from_le_bytes(value.as_slice().try_into().ok()?))
        } else {
            None
        }
    }
}

fn take<'a>(input: &mut &'a [u8], count: usize) -> Result<&'a [u8]> {
    let (head, rest) = input
        .split_at_checked(count)
        .ok_or("Truncated binary VDF")?;
    *input = rest;
    Ok(head)
}

fn cstring<'a>(input: &mut &'a [u8]) -> Result<&'a [u8]> {
    let length = input
        .iter()
        .position(|&b| b == 0)
        .ok_or("Unterminated binary VDF string")?;
    take(input, length + 1)
}
