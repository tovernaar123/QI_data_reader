use crate::error::QiError;
use pyo3::types::PyDict;
use pyo3::{prelude::*, types::PyString};
use pyo3_stub_gen::derive::*;
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Clone)]
pub enum Node {
    Map(HashMap<String, Node>),
    Value(String),
}

#[derive(Debug, Clone, Copy)]
pub enum NodeKind {
    Map,
    Value,
    Number,
}

impl Node {
    pub fn kind(&self) -> NodeKind {
        match self {
            Node::Map(_) => NodeKind::Map,
            Node::Value(_) => NodeKind::Value,
        }
    }

    fn as_map_mut(&mut self) -> Result<&mut HashMap<String, Node>, QiError> {
        match self {
            Node::Map(map) => Ok(map),
            other => Err(QiError::InvalidNodeVariant {
                exp: "map".to_string(),
                got: other.kind(),
            }),
        }
    }

    fn as_map(&self) -> Result<&HashMap<String, Node>, QiError> {
        match self {
            Node::Map(map) => Ok(map),
            other => Err(QiError::InvalidNodeVariant {
                exp: "map".to_string(),
                got: other.kind(),
            }),
        }
    }

    pub fn as_value(&self) -> Result<&String, QiError> {
        match self {
            Node::Value(map) => Ok(map),
            other => Err(QiError::InvalidNodeVariant {
                exp: "Value".to_string(),
                got: other.kind(),
            }),
        }
    }

    pub fn get(&self, name: &str) -> Result<&Node, QiError> {
        let res = self.as_map()?.get(name).ok_or(QiError::HeaderMissingKey {
            key: name.to_string(),
        })?;
        Ok(res)
    }
}

pub fn node_to_py(py: Python<'_>, node: &Node) -> PyResult<Py<PyAny>> {
    match node {
        Node::Value(s) => Ok(PyString::new(py, s).into()),

        Node::Map(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, node_to_py(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}

#[gen_stub_pyclass]
#[pyclass]
pub struct QiHeader {
    pub(crate) data: Node,
}

impl QiHeader {
    pub fn get_channels(&self) -> Result<Vec<(&str, &str)>, QiError> {
        let channel_map = self.data.get("lcd-info")?;
        let keys: Vec<&String> = channel_map.as_map()?.keys().collect();
        let mut names = vec![];
        for key in keys {
            let name = channel_map
                .get(key)?
                .get("channel")?
                .get("name")?
                .as_value()?;
            names.push((key.as_ref(), name.as_ref()));
        }
        return Ok(names);
    }

    fn get_conversion_rec(
        target: &Node,
        root: &Node,
        base: &str,
        voltoffset: f64,
        voltmult: f64,
    ) -> Result<(f64, f64), QiError> {
        let prev = target.get("base-calibration-slot")?;
        let offset: f64 = target.get("scaling")?.get("offset")?.as_value()?.parse()?;
        let multiplier: f64 = target
            .get("scaling")?
            .get("multiplier")?
            .as_value()?
            .parse()?;
        match prev.as_value()?.as_ref() {
            s if s == base => Ok((voltoffset * multiplier + offset, voltmult * multiplier)),
            other => {
                let new_target = root.get(other)?;
                let (new_offset, new_multiplier) =
                    QiHeader::get_conversion_rec(new_target, root, base, voltoffset, voltmult)?;

                // (x*new_multiplier + new_offset) * multiplier + offset;
                // x * (new_multiplier * multiplier) + new_offset* multiplier + offset
                Ok((
                    new_offset * multiplier + offset,
                    new_multiplier * multiplier,
                ))
            }
        }
    }

    pub fn convert_to_type(&self, name: &str, unit_type: &str) -> Result<(f64, f64, &str), QiError> {
        let id = self
            .get_channels()?
            .into_iter()
            .find(|a| a.1 == name)
            .ok_or(QiError::InvalidChannel {
                channel: name.to_string(),
            })?
            .0;

        let lcdinfo = self.data.get("lcd-info")?.get(id)?;
        let data_type = lcdinfo.get("type")?.as_value()?;

        let (offset, multiplier) = lcdinfo
            .get("encoder")
            .and_then(|n| n.get("scaling"))
            .and_then(|n| {
                let offset = n.get("offset")?.as_value()?.parse()?;
                let multiplier = n.get("multiplier")?.as_value()?.parse()?;
                Ok((offset, multiplier))
            })
            .unwrap_or((0.0, 1.0));
        let conversionset = lcdinfo.get("conversion-set")?;
        // lcd-info.6.conversion-set.conversions.base
        let base = conversionset.get("conversions")?.get("base")?.as_value()?;
        if base == unit_type
        {
            return Ok((offset, multiplier,data_type))
        }
        let root = conversionset.get("conversion")?;

        let target = root.get(unit_type)?;
        let (unitoffset, unitmult) =
            QiHeader::get_conversion_rec(target, root, base, offset, multiplier)?;
        println!("{},{}",unitoffset, unitmult);
        Ok((unitoffset, unitmult,data_type))
    }

    pub fn get_unit_types(&self, name: &str) -> Result<Vec<&str>, QiError> {
        let id = self
            .get_channels()?
            .into_iter()
            .find(|a| a.1 == name)
            .ok_or(QiError::InvalidChannel {
                channel: name.to_string(),
            })?
            .0;
        // conversion-set.conversion
        let channel_map = self
            .data
            .get("lcd-info")?
            .get(id)?
            .get("conversion-set")?
            .get("conversion")?;
        let keys: Vec<&str> = channel_map.as_map()?.keys().map(|f| f.as_ref()).collect();
        return Ok(keys);
    }
}

impl FromStr for QiHeader {
    type Err = QiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut hasmap: HashMap<String, Node> = HashMap::new();
        for line in s.split('\n').rev().skip(1) {
            if line.starts_with("#") {
                continue;
            }
            let (lhs, rhs) = line
                .split_once('=')
                .ok_or_else(|| QiError::InvalidHeaderLine {
                    line: line.to_string(),
                    reason: "missing '='",
                })?;
            let mut curs = &mut hasmap;
            let mut parts = lhs.split('.').peekable();
            while let Some(name) = parts.next() {
                if parts.peek().is_some() {
                    let x = curs
                        .entry(name.to_string())
                        .or_insert(Node::Map(HashMap::new()));
                    curs = x.as_map_mut()?;
                } else {
                    curs.insert(name.to_string(), Node::Value(rhs.to_string()));
                }
            }
        }

        return Ok(QiHeader {
            data: Node::Map(hasmap),
        });
    }
}
