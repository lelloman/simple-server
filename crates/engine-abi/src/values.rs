//! Lossless tagged SQLite values. This is a byte protocol, not a Rust ABI.
use alloc::{string::String, vec::Vec};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

fn bytes(out: &mut Vec<u8>, value: &[u8]) -> Result<(), &'static str> {
    let length = u32::try_from(value.len()).map_err(|_| "SQLite value is too large")?;
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(value);
    Ok(())
}
pub fn encode(values: &[Value]) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    out.extend_from_slice(
        &u32::try_from(values.len())
            .map_err(|_| "too many SQLite values")?
            .to_le_bytes(),
    );
    for value in values {
        match value {
            Value::Null => out.push(0),
            Value::Integer(n) => {
                out.push(1);
                out.extend_from_slice(&n.to_le_bytes());
            }
            Value::Real(n) => {
                out.push(2);
                out.extend_from_slice(&n.to_le_bytes());
            }
            Value::Text(s) => {
                out.push(3);
                bytes(&mut out, s.as_bytes())?;
            }
            Value::Blob(b) => {
                out.push(4);
                bytes(&mut out, b)?;
            }
        }
    }
    Ok(out)
}
fn take<'a>(input: &mut &'a [u8], length: usize) -> Result<&'a [u8], &'static str> {
    if input.len() < length {
        return Err("truncated SQLite value");
    }
    let (head, tail) = input.split_at(length);
    *input = tail;
    Ok(head)
}
pub fn decode(mut input: &[u8]) -> Result<Vec<Value>, &'static str> {
    let count = u32::from_le_bytes(take(&mut input, 4)?.try_into().unwrap()) as usize;
    if count > input.len() {
        return Err("invalid SQLite value count");
    }
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let tag = take(&mut input, 1)?[0];
        result.push(match tag {
            0 => Value::Null,
            1 => Value::Integer(i64::from_le_bytes(take(&mut input, 8)?.try_into().unwrap())),
            2 => Value::Real(f64::from_le_bytes(take(&mut input, 8)?.try_into().unwrap())),
            3 | 4 => {
                let length = u32::from_le_bytes(take(&mut input, 4)?.try_into().unwrap()) as usize;
                let bytes = take(&mut input, length)?.to_vec();
                if tag == 3 {
                    Value::Text(String::from_utf8(bytes).map_err(|_| "SQLite text is not UTF-8")?)
                } else {
                    Value::Blob(bytes)
                }
            }
            _ => return Err("invalid SQLite value tag"),
        });
    }
    if !input.is_empty() {
        return Err("trailing SQLite value bytes");
    }
    Ok(result)
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::Text(v.into())
    }
}
impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Self::Blob(v)
    }
}
impl From<&[u8]> for Value {
    fn from(v: &[u8]) -> Self {
        Self::Blob(v.into())
    }
}
impl<T: Clone + Into<Value>> From<&T> for Value {
    fn from(v: &T) -> Self {
        v.clone().into()
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        v.map_or(Self::Null, Into::into)
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Self::Integer(i64::from(v))
    }
}
macro_rules! integer { ($($ty:ty),*) => { $(impl From<$ty> for Value { fn from(v: $ty) -> Self { Self::Integer(i64::from(v)) } })* }; }
integer!(i8, i16, i32, i64, u8, u16, u32);
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Self::Real(v)
    }
}
impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Self::Real(v.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_values_and_invalid_frames() {
        let values = alloc::vec![
            Value::Integer(i64::MAX),
            Value::Integer(i64::MIN),
            Value::Real(-0.0),
            Value::Null,
            Value::Text("a\0é".into()),
            Value::Blob(alloc::vec![0, 255])
        ];
        let bytes = encode(&values).unwrap();
        assert_eq!(decode(&bytes).unwrap(), values);
        for length in 0..bytes.len() {
            assert!(decode(&bytes[..length]).is_err());
        }
        let mut extra = bytes;
        extra.push(0);
        assert!(decode(&extra).is_err());
    }
}
