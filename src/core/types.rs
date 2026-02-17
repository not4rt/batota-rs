use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
}

impl ValueType {
    pub fn size(&self) -> usize {
        match self {
            ValueType::I8 | ValueType::U8 => 1,
            ValueType::I16 | ValueType::U16 => 2,
            ValueType::I32 | ValueType::U32 | ValueType::F32 => 4,
            ValueType::I64 | ValueType::U64 | ValueType::F64 => 8,
        }
    }

    pub fn all() -> &'static [ValueType] {
        &[
            ValueType::I8,
            ValueType::I16,
            ValueType::I32,
            ValueType::I64,
            ValueType::U8,
            ValueType::U16,
            ValueType::U32,
            ValueType::U64,
            ValueType::F32,
            ValueType::F64,
        ]
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::I8 => write!(f, "Int8"),
            ValueType::I16 => write!(f, "Int16"),
            ValueType::I32 => write!(f, "Int32"),
            ValueType::I64 => write!(f, "Int64"),
            ValueType::U8 => write!(f, "UInt8"),
            ValueType::U16 => write!(f, "UInt16"),
            ValueType::U32 => write!(f, "UInt32"),
            ValueType::U64 => write!(f, "UInt64"),
            ValueType::F32 => write!(f, "Float"),
            ValueType::F64 => write!(f, "Double"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanType {
    ExactValue,
    GreaterThan,
    LessThan,
    UnknownInitial,
    IncreasedValue,
    DecreasedValue,
    ChangedValue,
    UnchangedValue,
}

impl fmt::Display for ScanType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanType::ExactValue => write!(f, "Exact Value"),
            ScanType::GreaterThan => write!(f, "Greater Than"),
            ScanType::LessThan => write!(f, "Less Than"),
            ScanType::UnknownInitial => write!(f, "Unknown Initial Value"),
            ScanType::IncreasedValue => write!(f, "Increased Value"),
            ScanType::DecreasedValue => write!(f, "Decreased Value"),
            ScanType::ChangedValue => write!(f, "Changed Value"),
            ScanType::UnchangedValue => write!(f, "Unchanged Value"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
}

impl Value {
    pub fn from_bytes(bytes: &[u8], value_type: ValueType) -> Option<Self> {
        match value_type {
            ValueType::I8 => Some(Value::I8(i8::from_le_bytes([bytes[0]]))),
            ValueType::I16 => Some(Value::I16(i16::from_le_bytes(bytes.try_into().ok()?))),
            ValueType::I32 => Some(Value::I32(i32::from_le_bytes(bytes.try_into().ok()?))),
            ValueType::I64 => Some(Value::I64(i64::from_le_bytes(bytes.try_into().ok()?))),
            ValueType::U8 => Some(Value::U8(u8::from_le_bytes([bytes[0]]))),
            ValueType::U16 => Some(Value::U16(u16::from_le_bytes(bytes.try_into().ok()?))),
            ValueType::U32 => Some(Value::U32(u32::from_le_bytes(bytes.try_into().ok()?))),
            ValueType::U64 => Some(Value::U64(u64::from_le_bytes(bytes.try_into().ok()?))),
            ValueType::F32 => Some(Value::F32(f32::from_le_bytes(bytes.try_into().ok()?))),
            ValueType::F64 => Some(Value::F64(f64::from_le_bytes(bytes.try_into().ok()?))),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Value::I8(v) => v.to_le_bytes().to_vec(),
            Value::I16(v) => v.to_le_bytes().to_vec(),
            Value::I32(v) => v.to_le_bytes().to_vec(),
            Value::I64(v) => v.to_le_bytes().to_vec(),
            Value::U8(v) => v.to_le_bytes().to_vec(),
            Value::U16(v) => v.to_le_bytes().to_vec(),
            Value::U32(v) => v.to_le_bytes().to_vec(),
            Value::U64(v) => v.to_le_bytes().to_vec(),
            Value::F32(v) => v.to_le_bytes().to_vec(),
            Value::F64(v) => v.to_le_bytes().to_vec(),
        }
    }

    pub fn compare(&self, other: &Value, scan_type: ScanType) -> bool {
        match scan_type {
            ScanType::ExactValue => self.eq_value(other),
            ScanType::GreaterThan => self.gt_value(other),
            ScanType::LessThan => self.lt_value(other),
            ScanType::IncreasedValue => self.gt_value(other),
            ScanType::DecreasedValue => self.lt_value(other),
            ScanType::ChangedValue => !self.eq_value(other),
            ScanType::UnchangedValue => self.eq_value(other),
            ScanType::UnknownInitial => true,
        }
    }

    fn eq_value(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::I8(a), Value::I8(b)) => a == b,
            (Value::I16(a), Value::I16(b)) => a == b,
            (Value::I32(a), Value::I32(b)) => a == b,
            (Value::I64(a), Value::I64(b)) => a == b,
            (Value::U8(a), Value::U8(b)) => a == b,
            (Value::U16(a), Value::U16(b)) => a == b,
            (Value::U32(a), Value::U32(b)) => a == b,
            (Value::U64(a), Value::U64(b)) => a == b,
            (Value::F32(a), Value::F32(b)) => (a - b).abs() < f32::EPSILON,
            (Value::F64(a), Value::F64(b)) => (a - b).abs() < f64::EPSILON,
            _ => false,
        }
    }

    fn gt_value(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::I8(a), Value::I8(b)) => a > b,
            (Value::I16(a), Value::I16(b)) => a > b,
            (Value::I32(a), Value::I32(b)) => a > b,
            (Value::I64(a), Value::I64(b)) => a > b,
            (Value::U8(a), Value::U8(b)) => a > b,
            (Value::U16(a), Value::U16(b)) => a > b,
            (Value::U32(a), Value::U32(b)) => a > b,
            (Value::U64(a), Value::U64(b)) => a > b,
            (Value::F32(a), Value::F32(b)) => a > b,
            (Value::F64(a), Value::F64(b)) => a > b,
            _ => false,
        }
    }

    fn lt_value(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::I8(a), Value::I8(b)) => a < b,
            (Value::I16(a), Value::I16(b)) => a < b,
            (Value::I32(a), Value::I32(b)) => a < b,
            (Value::I64(a), Value::I64(b)) => a < b,
            (Value::U8(a), Value::U8(b)) => a < b,
            (Value::U16(a), Value::U16(b)) => a < b,
            (Value::U32(a), Value::U32(b)) => a < b,
            (Value::U64(a), Value::U64(b)) => a < b,
            (Value::F32(a), Value::F32(b)) => a < b,
            (Value::F64(a), Value::F64(b)) => a < b,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I8(v) => write!(f, "{}", v),
            Value::I16(v) => write!(f, "{}", v),
            Value::I32(v) => write!(f, "{}", v),
            Value::I64(v) => write!(f, "{}", v),
            Value::U8(v) => write!(f, "{}", v),
            Value::U16(v) => write!(f, "{}", v),
            Value::U32(v) => write!(f, "{}", v),
            Value::U64(v) => write!(f, "{}", v),
            Value::F32(v) => write!(f, "{}", v),
            Value::F64(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FoundAddress {
    pub address: usize,
    pub value: Value,
}
