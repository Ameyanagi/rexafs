//! Numeric array layouts used by Larch's jsonutils/npjson serializers.
//! Original dtype bytes remain available even when f64 views round integers.
use super::super::{MeasurementDataset, ReadError, MAX_BYTES};
use super::{error, json::parse as json};
use base64::Engine;
use serde_json::Value;
use std::collections::BTreeMap;

struct Dtype {
    kind: u8,
    width: usize,
    big: bool,
    descriptor: String,
}
impl Dtype {
    fn parse(name: &str, path: &str) -> Result<Self, ReadError> {
        let aliases = [
            ("bool", "|b1"),
            ("bool_", "|b1"),
            ("int8", "|i1"),
            ("int16", "<i2"),
            ("int32", "<i4"),
            ("int64", "<i8"),
            ("uint8", "|u1"),
            ("uint16", "<u2"),
            ("uint32", "<u4"),
            ("uint64", "<u8"),
            ("float32", "<f4"),
            ("float64", "<f8"),
            ("complex64", "<c8"),
            ("complex128", "<c16"),
        ];
        let descriptor = aliases
            .iter()
            .find(|(alias, _)| *alias == name)
            .map(|(_, d)| *d)
            .unwrap_or(name);
        let bytes = descriptor.as_bytes();
        let (order, offset) = if bytes.first().is_some_and(|c| b"<>|=".contains(c)) {
            (bytes[0], 1)
        } else {
            (b'=', 0)
        };
        let kind = *bytes
            .get(offset)
            .ok_or_else(|| error(path, "missing array dtype"))?;
        let width = descriptor
            .get(offset + 1..)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let valid = match kind {
            b'b' => width == 1,
            b'i' | b'u' => matches!(width, 1 | 2 | 4 | 8),
            b'f' => matches!(width, 4 | 8),
            b'c' => matches!(width, 8 | 16),
            _ => false,
        };
        if !valid || (width > 1 && matches!(order, b'|' | b'=')) {
            return Err(error(path, format!("unsupported or ambiguous array dtype {name:?}; expected explicit-endian numeric data")));
        }
        Ok(Self {
            kind,
            width,
            big: order == b'>',
            descriptor: descriptor.into(),
        })
    }
    fn complex(&self) -> bool {
        self.kind == b'c'
    }
    fn component_width(&self) -> usize {
        if self.complex() {
            self.width / 2
        } else {
            self.width
        }
    }

    fn number(&self, bytes: &[u8]) -> (f64, bool) {
        let mut padded = [0u8; 8];
        for (i, &byte) in bytes.iter().enumerate() {
            padded[if self.big { bytes.len() - i - 1 } else { i }] = byte;
        }
        let bits = u64::from_le_bytes(padded);
        match self.kind {
            b'b' => (f64::from(bits != 0), false),
            b'i' => {
                let shift = 64 - bytes.len() * 8;
                let value = ((bits << shift) as i64) >> shift;
                let view = value as f64;
                (view, view as i128 != i128::from(value))
            }
            b'u' => {
                let view = bits as f64;
                (view, view as u128 != u128::from(bits))
            }
            _ if bytes.len() == 4 => (f32::from_bits(bits as u32) as f64, false),
            _ => (f64::from_bits(bits), false),
        }
    }

    fn append_legacy(
        &self,
        value: &Value,
        bytes: &mut Vec<u8>,
        path: &str,
    ) -> Result<(), ReadError> {
        let bad = || error(path, "legacy array value does not match its dtype");
        let width = self.component_width();
        let bits = match self.kind {
            b'b' => u64::from(value.as_bool().ok_or_else(bad)?),
            b'i' => {
                let n = value.as_i64().ok_or_else(bad)?;
                if width < 8 && (n < -(1i64 << (width * 8 - 1)) || n >= (1i64 << (width * 8 - 1))) {
                    return Err(bad());
                }
                n as u64
            }
            b'u' => {
                let n = value.as_u64().ok_or_else(bad)?;
                if width < 8 && n >= 1u64 << (width * 8) {
                    return Err(bad());
                }
                n
            }
            _ => {
                let n = value
                    .as_f64()
                    .or_else(|| match value.as_str()? {
                        "NaN" => Some(f64::NAN),
                        "Infinity" => Some(f64::INFINITY),
                        "-Infinity" => Some(f64::NEG_INFINITY),
                        _ => None,
                    })
                    .ok_or_else(bad)?;
                if width == 4 {
                    u64::from((n as f32).to_bits())
                } else {
                    n.to_bits()
                }
            }
        };
        let word = bits.to_le_bytes();
        if self.big {
            bytes.extend(word[..width].iter().rev());
        } else {
            bytes.extend(&word[..width]);
        }
        Ok(())
    }
}

pub(super) fn decode(
    value: &Value,
    path: &str,
    budget: &mut usize,
) -> Result<(MeasurementDataset, bool), ReadError> {
    let modern = value["__class__"] == "b64ndarray";
    let inner = if modern {
        json(
            value["__value__"]
                .as_str()
                .ok_or_else(|| error(path, "array __value__ must contain JSON text"))?,
            path,
        )?
    } else {
        Value::Null
    };
    if modern && inner["_type_"] != "b64ndarray" {
        return Err(error(path, "invalid inner array type"));
    }
    let array = if modern { &inner } else { value };
    let shape_key = if modern { "shape" } else { "__shape__" };
    let dtype_key = if modern { "dtype" } else { "__dtype__" };
    let shape: Vec<u64> = array[shape_key]
        .as_array()
        .filter(|s| s.len() <= 32)
        .ok_or_else(|| error(path, "array shape must have at most 32 dimensions"))?
        .iter()
        .map(|n| {
            n.as_u64()
                .ok_or_else(|| error(path, "array dimensions must be nonnegative integers"))
        })
        .collect::<Result<_, _>>()?;
    let dtype_name = array[dtype_key]
        .as_str()
        .ok_or_else(|| error(path, "array dtype must be text"))?;
    if modern && !dtype_name.starts_with(['<', '>', '|']) {
        return Err(error(
            path,
            "binary array dtype must declare its byte order",
        ));
    }
    let dtype = Dtype::parse(dtype_name, path)?;
    let count = shape
        .iter()
        .try_fold(1usize, |a, &b| {
            usize::try_from(b).ok().and_then(|b| a.checked_mul(b))
        })
        .ok_or_else(|| error(path, "array shape product overflows"))?;
    let bytes_len = count
        .checked_mul(dtype.width)
        .ok_or_else(|| error(path, "array byte count overflows"))?;
    let view_len = count
        .checked_mul(if dtype.complex() { 16 } else { 8 })
        .ok_or_else(|| error(path, "array view size overflows"))?;
    let total = budget
        .checked_add(bytes_len)
        .and_then(|n| n.checked_add(view_len))
        .filter(|&n| n <= MAX_BYTES)
        .ok_or_else(|| error(path, "decoded arrays exceed the 256 MiB budget"))?;
    *budget = total;
    let bytes = if modern {
        let encoded = array["value"]
            .as_str()
            .ok_or_else(|| error(path, "array base64 payload must be text"))?;
        if encoded.len() != bytes_len.div_ceil(3) * 4 {
            return Err(error(
                path,
                "array shape/dtype disagree with base64 payload length",
            ));
        }
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| error(path, format!("invalid array base64: {e}")))?
    } else {
        let values = array["value"]
            .as_array()
            .ok_or_else(|| error(path, "legacy array requires numeric lists"))?;
        let (real, imag) = if dtype.complex() {
            if values.len() != 2 {
                return Err(error(
                    path,
                    "complex legacy array requires real and imaginary lists",
                ));
            }
            let real = values[0]
                .as_array()
                .ok_or_else(|| error(path, "invalid legacy real array"))?;
            let imag = values[1]
                .as_array()
                .ok_or_else(|| error(path, "invalid legacy imaginary array"))?;
            (real, Some(imag))
        } else {
            (values, None)
        };
        if real.len() != count || imag.is_some_and(|v| v.len() != count) {
            return Err(error(
                path,
                "legacy array shape disagrees with payload length",
            ));
        }
        let mut bytes = Vec::with_capacity(bytes_len);
        for (i, value) in real.iter().enumerate() {
            dtype.append_legacy(value, &mut bytes, path)?;
            if let Some(imag) = imag {
                dtype.append_legacy(&imag[i], &mut bytes, path)?;
            }
        }
        bytes
    };
    if bytes.len() != bytes_len {
        return Err(error(
            path,
            "array shape/dtype disagree with decoded byte length",
        ));
    }
    let mut values = Vec::with_capacity(count);
    let mut imaginary = dtype.complex().then(|| Vec::with_capacity(count));
    let mut imprecise = false;
    for word in bytes.chunks_exact(dtype.width) {
        let (value, rounded) = dtype.number(&word[..dtype.component_width()]);
        values.push(value);
        imprecise |= rounded;
        if let Some(imag) = &mut imaginary {
            imag.push(dtype.number(&word[dtype.component_width()..]).0);
        }
    }
    let attributes = BTreeMap::from([
        ("larix.dtype".into(), dtype.descriptor),
        (
            "larix.encoding".into(),
            if modern { "b64ndarray" } else { "Array" }.into(),
        ),
        (
            "larix.bytes_base64".into(),
            base64::engine::general_purpose::STANDARD.encode(&bytes),
        ),
        (
            "quantity".into(),
            format!(
                "Stored Larix array {}",
                path.rsplit('/').next().unwrap_or(path)
            ),
        ),
    ]);
    Ok((
        MeasurementDataset {
            path: path.into(),
            shape,
            values,
            imaginary,
            attributes,
        },
        imprecise,
    ))
}
